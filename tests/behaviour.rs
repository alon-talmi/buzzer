use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct World {
    _temp: TempDir,
    home: PathBuf,
    repo: PathBuf,
    worktree: PathBuf,
    other_repo: PathBuf,
    children: Vec<Child>,
}

fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git").args(args).current_dir(cwd).output().unwrap().status;
    assert!(status.success(), "git {args:?} failed");
}

impl World {
    fn new() -> World {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let repo = root.join("repo");
        let other_repo = root.join("other");
        for folder in [&repo, &other_repo] {
            std::fs::create_dir_all(folder).unwrap();
            git(folder, &["init", "-q"]);
            git(folder, &["-c", "user.email=a@b", "-c", "user.name=t", "commit", "-q", "--allow-empty", "-m", "x"]);
        }
        let worktree = root.join("repo-wt");
        git(&repo, &["worktree", "add", "-q", worktree.to_str().unwrap()]);
        World { home: root.join("state"), _temp: temp, repo, worktree, other_repo, children: vec![] }
    }

    fn command(&self, cwd: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_buzzer"));
        command.args(args).current_dir(cwd).env("BUZZER_HOME", &self.home);
        command
    }

    fn cli(&self, args: &[&str]) -> Output {
        self.cli_in(&self.repo, args)
    }

    fn cli_in(&self, cwd: &Path, args: &[&str]) -> Output {
        self.command(cwd, args).output().unwrap()
    }

    fn start(&mut self, cwd: &Path, args: &[&str]) -> usize {
        let child = self.command(cwd, args).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        self.children.push(child);
        self.children.len() - 1
    }

    fn running(&mut self, index: usize) -> bool {
        self.children[index].try_wait().unwrap().is_none()
    }

    fn finishes(&mut self, index: usize) -> bool {
        wait_until(|| self.children[index].try_wait().unwrap().is_some())
    }

    fn status(&self, queue: &str, cwd: &Path, flags: &[&str]) -> String {
        let mut args = vec!["status", queue];
        args.extend(flags);
        String::from_utf8(self.cli_in(cwd, &args).stdout).unwrap()
    }

    fn waiting(&self, queue: &str) -> usize {
        self.status(queue, &self.repo, &[]).matches("waiting ").count()
    }

    fn done(&self, queue: &str) -> Output {
        let status = self.status(queue, &self.repo, &[]);
        let ticket = status.split("HOLDS").nth(1).unwrap().split("ticket ").nth(1).unwrap().split(',').next().unwrap();
        self.cli(&["done", queue, "--ticket", ticket])
    }
}

impl Drop for World {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs(8);
    while Instant::now() < end {
        if condition() {
            return true;
        }
        sleep(Duration::from_millis(50));
    }
    false
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn first_taker_gets_it_at_once() {
    let world = World::new();
    let result = world.cli(&["wait", "device", "--label", "a"]);
    assert!(result.status.success());
    assert!(text(&result).contains("YOUR TURN"));
    assert!(world.status("device", &world.repo, &[]).contains("HOLDS   a"));
}

#[test]
fn second_waits_until_done_then_wakes() {
    let mut world = World::new();
    world.cli(&["wait", "device"]);
    let repo = world.repo.clone();
    let second = world.start(&repo, &["wait", "device", "--label", "b"]);
    assert!(wait_until(|| world.waiting("device") == 1));
    assert!(world.running(second));
    world.done("device");
    assert!(world.finishes(second));
}

#[test]
fn fifo_order() {
    let mut world = World::new();
    world.cli(&["wait", "device"]);
    let repo = world.repo.clone();
    let mut waiters = vec![];
    for (n, label) in ["b", "c", "d"].iter().enumerate() {
        waiters.push(world.start(&repo, &["wait", "device", "--label", label]));
        assert!(wait_until(|| world.waiting("device") == n + 1));
    }
    for waiter in waiters {
        assert!(world.running(waiter));
        world.done("device");
        assert!(world.finishes(waiter));
    }
}

#[test]
fn jump_goes_ahead_of_waiters_but_not_the_holder() {
    let mut world = World::new();
    world.cli(&["wait", "device", "--label", "holder"]);
    let repo = world.repo.clone();
    let slow = world.start(&repo, &["wait", "device", "--label", "slow"]);
    assert!(wait_until(|| world.waiting("device") == 1));
    let urgent = world.start(&repo, &["wait", "device", "--first", "--label", "urgent"]);
    assert!(wait_until(|| world.waiting("device") == 2));
    assert!(world.status("device", &repo, &[]).contains("HOLDS   holder"));
    world.done("device");
    assert!(world.finishes(urgent));
    assert!(world.running(slow));
}

#[test]
fn worktrees_share_project_queue() {
    let mut world = World::new();
    world.cli(&["wait", "editor"]);
    let worktree = world.worktree.clone();
    let second = world.start(&worktree, &["wait", "editor"]);
    assert!(wait_until(|| world.waiting("editor") == 1));
    assert!(world.running(second));
}

#[test]
fn other_project_has_its_own_queue() {
    let world = World::new();
    world.cli(&["wait", "editor"]);
    let other = world.cli_in(&world.other_repo, &["wait", "editor"]);
    assert!(other.status.success());
}

#[test]
fn global_scope_is_shared_across_projects_and_separate_from_project() {
    let mut world = World::new();
    world.cli(&["wait", "phone", "-g"]);
    let other_repo = world.other_repo.clone();
    let blocked = world.start(&other_repo, &["wait", "phone", "-g"]);
    assert!(wait_until(|| world.status("phone", &world.repo, &["-g"]).contains("waiting ")));
    assert!(world.running(blocked));
    assert!(world.cli(&["wait", "phone"]).status.success());
}

#[test]
fn waiter_that_dies_loses_its_place() {
    let mut world = World::new();
    world.cli(&["wait", "device"]);
    let repo = world.repo.clone();
    let doomed = world.start(&repo, &["wait", "device", "--label", "doomed"]);
    assert!(wait_until(|| world.waiting("device") == 1));
    world.children[doomed].kill().unwrap();
    world.children[doomed].wait().unwrap();
    let survivor = world.start(&repo, &["wait", "device", "--label", "survivor"]);
    assert!(wait_until(|| world.status("device", &world.repo, &[]).contains("survivor")));
    world.done("device");
    assert!(world.finishes(survivor));
    assert!(!world.status("device", &repo, &[]).contains("doomed"));
}

#[test]
fn terminated_waiter_cleans_its_ticket() {
    let mut world = World::new();
    world.cli(&["wait", "device"]);
    let repo = world.repo.clone();
    let waiter = world.start(&repo, &["wait", "device"]);
    assert!(wait_until(|| world.waiting("device") == 1));
    Command::new("kill").arg(world.children[waiter].id().to_string()).status().unwrap();
    assert!(world.finishes(waiter));
    assert_eq!(world.waiting("device"), 0);
}

#[test]
fn run_releases_when_command_ends() {
    let world = World::new();
    let result = world.cli(&["run", "device", "--", "echo", "hi"]);
    assert!(result.status.success());
    assert!(text(&result).contains("hi"));
    assert!(world.status("device", &world.repo, &[]).contains("(idle)"));
}

#[test]
fn wait_turn_ends_when_the_owning_session_dies() {
    let world = World::new();
    let mut session = Command::new("sleep").arg("60").spawn().unwrap();
    let held = world
        .command(&world.repo, &["wait", "device"])
        .env("BUZZER_OWNER_PID", session.id().to_string())
        .output()
        .unwrap();
    assert!(held.status.success());
    assert!(world.status("device", &world.repo, &[]).contains("HOLDS"));
    session.kill().unwrap();
    session.wait().unwrap();
    assert!(world.status("device", &world.repo, &[]).contains("(idle)"));
}

#[test]
fn run_passes_exit_code_through() {
    let world = World::new();
    let result = world.cli(&["run", "device", "--", "sh", "-c", "exit 7"]);
    assert_eq!(result.status.code(), Some(7));
    assert!(world.status("device", &world.repo, &[]).contains("(idle)"));
}

#[test]
fn run_holder_killed_frees_queue() {
    let mut world = World::new();
    let repo = world.repo.clone();
    let holder = world.start(&repo, &["run", "device", "--", "sleep", "60"]);
    assert!(wait_until(|| world.status("device", &world.repo, &[]).contains("HOLDS")));
    world.children[holder].kill().unwrap();
    world.children[holder].wait().unwrap();
    assert!(world.cli(&["wait", "device"]).status.success());
}

#[test]
fn hold_limit_frees_a_forgotten_hold() {
    let world = World::new();
    world.cli(&["wait", "device", "--hold", "1s"]);
    sleep(Duration::from_millis(2100));
    assert!(world.cli(&["wait", "device"]).status.success());
}

#[test]
fn renew_extends_hold() {
    let world = World::new();
    world.cli(&["wait", "device", "--hold", "1s"]);
    assert!(world.cli(&["renew", "device", "--hold", "1h"]).status.success());
    sleep(Duration::from_millis(2100));
    assert!(world.status("device", &world.repo, &[]).contains("HOLDS"));
}

#[test]
fn kick_removes_a_ticket() {
    let world = World::new();
    world.cli(&["wait", "device"]);
    let status = world.status("device", &world.repo, &[]);
    let ticket = status.split("ticket ").nth(1).unwrap().split(',').next().unwrap().to_string();
    assert!(world.cli(&["kick", "device", "--ticket", &ticket]).status.success());
    assert!(world.status("device", &world.repo, &[]).contains("(idle)"));
}

#[test]
fn done_with_wrong_ticket_is_refused() {
    let world = World::new();
    world.cli(&["wait", "device"]);
    assert!(!world.cli(&["done", "device", "--ticket", "nope"]).status.success());
    assert!(world.status("device", &world.repo, &[]).contains("HOLDS"));
    assert!(!world.cli(&["done", "device"]).status.success());
}

#[test]
fn bad_queue_name_is_refused() {
    let world = World::new();
    assert!(!world.cli(&["wait", "../evil"]).status.success());
}

#[test]
fn guards_list_project_and_global_rules_in_plain_words() {
    let world = World::new();
    assert!(world.cli(&["guard", "staging-server", "Only one person deploys at a time."]).status.success());
    assert!(world.cli(&["guard", "label-printer", "Shared printer.", "-g"]).status.success());
    let listing = text(&world.cli(&["guards"]));
    assert!(listing.contains("staging-server  [project:repo]  Only one person deploys at a time."), "{listing}");
    assert!(listing.contains("label-printer -g  [global]  Shared printer."), "{listing}");
    let elsewhere = text(&world.cli_in(&world.other_repo, &["guards"]));
    assert!(!elsewhere.contains("staging-server"));
    assert!(elsewhere.contains("label-printer"));
}

#[test]
fn guards_quiet_if_empty_prints_nothing_when_empty() {
    let world = World::new();
    assert_eq!(text(&world.cli(&["guards", "--quiet-if-empty"])), "");
    assert!(text(&world.cli(&["guards"])).contains("nothing is listed"));
}

#[test]
fn unguard_removes_an_entry() {
    let world = World::new();
    world.cli(&["guard", "brand-deck", "One editor at a time."]);
    assert!(world.cli(&["unguard", "brand-deck"]).status.success());
    assert!(!text(&world.cli(&["guards"])).contains("brand-deck"));
    assert!(!world.cli(&["unguard", "brand-deck"]).status.success());
}

#[test]
fn guard_does_not_break_the_queue_listing() {
    let world = World::new();
    world.cli(&["guard", "deck", "One editor."]);
    world.cli(&["wait", "deck"]);
    assert!(text(&world.cli(&["status"])).contains("project:repo/deck"));
}

#[test]
fn guide_prints() {
    let world = World::new();
    assert!(text(&world.cli(&["guide"])).contains("# buzzer guide"));
}

#[test]
fn status_of_a_ticket_from_an_older_version_shows_sane_times() {
    let world = World::new();
    world.cli(&["wait", "device"]);
    let tickets = std::fs::read_dir(world.home.join("projects")).unwrap().flatten().next().unwrap().path().join("device/tickets");
    let file = std::fs::read_dir(tickets).unwrap().flatten().next().unwrap().path();
    let old = std::fs::read_to_string(&file).unwrap().replace("\"taken\":", "\"x_taken\":").replace("\"renewed\":", "\"x_renewed\":");
    std::fs::write(&file, old).unwrap();
    let status = world.status("device", &world.repo, &[]);
    assert!(status.contains("held 0s") || status.contains("held 1s"), "{status}");
}
