use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const POLL: Duration = Duration::from_millis(250);
const DEFAULT_HOLD: &str = "2h";

#[derive(Parser)]
#[command(name = "buzzer", version, about = "Wait your turn for a shared thing, then get buzzed.")]
struct Cli {
    #[command(subcommand)]
    verb: Verb,
}

#[derive(Args)]
struct Where {
    /// Queue name (letters, digits, . _ -)
    queue: String,
    /// Use the machine-wide scope instead of this project's
    #[arg(short = 'g', long = "global")]
    global: bool,
}

#[derive(Args)]
struct JoinOptions {
    /// Jump ahead of everyone waiting (never preempts the current holder)
    #[arg(long)]
    first: bool,
    /// Who you are, shown in status
    #[arg(long)]
    label: Option<String>,
    #[arg(short, long)]
    quiet: bool,
    /// Longest you may hold it before it frees itself
    #[arg(long, default_value = DEFAULT_HOLD)]
    hold: String,
}

#[derive(Subcommand)]
enum Verb {
    /// Block until it is your turn, then exit 0 (you now hold it)
    Wait {
        #[command(flatten)]
        at: Where,
        #[command(flatten)]
        join: JoinOptions,
    },
    /// Wait, run a command, release when it ends: buzzer run NAME -- cmd ...
    Run {
        #[command(flatten)]
        at: Where,
        #[command(flatten)]
        join: JoinOptions,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1.., required = true)]
        command: Vec<String>,
    },
    /// Release the queue you hold (only the holder's ticket works)
    Done {
        #[command(flatten)]
        at: Where,
        /// The ticket id printed when it became your turn
        #[arg(long)]
        ticket: String,
    },
    /// Restart the hold timer
    Renew {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        hold: Option<String>,
    },
    /// Remove a ticket (stuck holder or waiter)
    Kick {
        #[command(flatten)]
        at: Where,
        #[arg(long)]
        ticket: String,
    },
    /// Show a queue, or every queue in the scope
    Status {
        queue: Option<String>,
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
}

#[derive(Serialize, Deserialize, Clone)]
struct Ticket {
    id: String,
    pid: u32,
    held: bool,
    jumped: bool,
    created: u64,
    label: String,
    cwd: String,
    hold: u64,
    #[serde(default)]
    owner_pid: Option<u32>,
    #[serde(default)]
    expires: u64,
}

struct Queue {
    dir: PathBuf,
    name: String,
}

struct Lock(File);

impl Drop for Lock {
    fn drop(&mut self) {
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

fn fail(message: &str) -> ! {
    eprintln!("buzzer: {message}");
    process::exit(1);
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn now_nanos() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
}

fn parse_duration(text: &str) -> u64 {
    let text = text.trim();
    let split = text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
    let (digits, unit) = text.split_at(split);
    let factor = match unit {
        "" | "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86400,
        _ => fail(&format!("bad duration '{text}' (use 90s, 30m, 2h, 1d)")),
    };
    match digits.parse::<u64>() {
        Ok(n) => n * factor,
        Err(_) => fail(&format!("bad duration '{text}' (use 90s, 30m, 2h, 1d)")),
    }
}

fn state_root() -> PathBuf {
    if let Some(home) = std::env::var_os("BUZZER_HOME") {
        return PathBuf::from(home);
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state/buzzer")
}

fn stable_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn project_identity() -> (String, String) {
    if let Ok(over) = std::env::var("BUZZER_PROJECT") {
        let name = Path::new(over.trim_end_matches('/')).file_name().map(|n| n.to_string_lossy().to_string());
        return (over.clone(), name.unwrap_or_else(|| "project".into()));
    }
    let from_git = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string());
    let root = match from_git {
        Some(common) => {
            let common = PathBuf::from(common);
            if common.file_name().is_some_and(|n| n == ".git") {
                common.parent().unwrap().to_path_buf()
            } else {
                common
            }
        }
        None => std::env::current_dir().unwrap(),
    };
    let root = fs::canonicalize(&root).unwrap_or(root);
    let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "project".into());
    (root.to_string_lossy().to_string(), name)
}

fn scope_dir(global: bool) -> (PathBuf, String) {
    if global {
        return (state_root().join("global"), "global".into());
    }
    let (root, name) = project_identity();
    let folder = format!("{name}-{:08x}", stable_hash(&root) & 0xffff_ffff);
    (state_root().join("projects").join(folder), format!("project:{name}"))
}

fn valid_queue_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn open_queue(queue: &str, global: bool) -> Queue {
    if !valid_queue_name(queue) {
        fail(&format!("bad queue name '{queue}' (letters, digits, . _ -)"));
    }
    let (base, label) = scope_dir(global);
    let dir = base.join(queue);
    fs::create_dir_all(dir.join("tickets")).unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", dir.display())));
    Queue { dir, name: format!("{label}/{queue}") }
}

fn lock(queue: &Queue) -> Lock {
    let file = File::create(queue.dir.join("lock")).unwrap_or_else(|e| fail(&format!("cannot open lock: {e}")));
    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    Lock(file)
}

fn pid_alive(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as i32, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn read_tickets(queue: &Queue) -> Vec<Ticket> {
    let Ok(entries) = fs::read_dir(queue.dir.join("tickets")) else { return vec![] };
    entries
        .flatten()
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str(&text).ok())
        .collect()
}

fn ticket_path(queue: &Queue, id: &str) -> PathBuf {
    queue.dir.join("tickets").join(format!("{id}.json"))
}

fn write_ticket(queue: &Queue, ticket: &Ticket) {
    let target = ticket_path(queue, &ticket.id);
    let temp = target.with_extension("tmp");
    let mut file = File::create(&temp).unwrap_or_else(|e| fail(&format!("cannot write ticket: {e}")));
    file.write_all(serde_json::to_string(ticket).unwrap().as_bytes()).unwrap();
    fs::rename(&temp, &target).unwrap_or_else(|e| fail(&format!("cannot write ticket: {e}")));
}

fn remove_ticket(queue: &Queue, id: &str) {
    let _ = fs::remove_file(ticket_path(queue, id));
}

fn is_dead(ticket: &Ticket, now: u64) -> bool {
    if !ticket.held {
        return !pid_alive(ticket.pid);
    }
    ticket.expires <= now || ticket.owner_pid.is_some_and(|pid| !pid_alive(pid))
}

fn reap(queue: &Queue, now: u64) {
    for ticket in read_tickets(queue) {
        if is_dead(&ticket, now) {
            remove_ticket(queue, &ticket.id);
        }
    }
}

fn line(queue: &Queue) -> Vec<Ticket> {
    let mut waiting: Vec<Ticket> = read_tickets(queue).into_iter().filter(|t| !t.held).collect();
    waiting.sort_by(|a, b| (!a.jumped, a.created, &a.id).cmp(&(!b.jumped, b.created, &b.id)));
    waiting
}

fn holder(queue: &Queue) -> Option<Ticket> {
    read_tickets(queue).into_iter().find(|t| t.held)
}

fn join(queue: &Queue, options: &JoinOptions) -> Ticket {
    let ticket = Ticket {
        id: format!("{}-{}", now_nanos(), process::id()),
        pid: process::id(),
        held: false,
        jumped: options.first,
        created: now_nanos(),
        label: options.label.clone().unwrap_or_default(),
        cwd: std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        hold: parse_duration(&options.hold),
        owner_pid: None,
        expires: 0,
    };
    let _guard = lock(queue);
    write_ticket(queue, &ticket);
    ticket
}

fn take_turn_if_first(queue: &Queue, ticket: &mut Ticket, owner_pid: Option<u32>) -> bool {
    let _guard = lock(queue);
    reap(queue, now_secs());
    let first = line(queue).into_iter().next();
    if first.is_some_and(|f| f.id == ticket.id) && holder(queue).is_none() {
        ticket.held = true;
        ticket.owner_pid = owner_pid;
        ticket.expires = now_secs() + ticket.hold;
        write_ticket(queue, ticket);
        return true;
    }
    false
}

fn place_in_line(queue: &Queue, ticket: &Ticket) -> usize {
    let _guard = lock(queue);
    line(queue).iter().position(|t| t.id == ticket.id).map_or(1, |i| i + 1)
}

fn parent_and_command(pid: u32) -> Option<(u32, String)> {
    let out = Command::new("ps").args(["-o", "ppid=,command=", "-p", &pid.to_string()]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let (parent, command) = text.split_once(' ')?;
    Some((parent.trim().parse().ok()?, command.to_string()))
}

/// The agent session that asked for the turn: BUZZER_OWNER_PID, else the nearest ancestor named like claude.
fn detect_session_pid() -> Option<u32> {
    if let Some(pid) = std::env::var("BUZZER_OWNER_PID").ok().and_then(|v| v.parse().ok()) {
        return Some(pid);
    }
    let mut pid = std::process::id();
    for _ in 0..12 {
        let (parent, _) = parent_and_command(pid)?;
        if parent <= 1 {
            return None;
        }
        let (_, command) = parent_and_command(parent)?;
        if command.to_lowercase().contains("claude") {
            return Some(parent);
        }
        pid = parent;
    }
    None
}

fn register_stop_flag() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    for signal in [SIGTERM, SIGINT, SIGHUP] {
        signal_hook::flag::register(signal, Arc::clone(&flag)).unwrap();
    }
    flag
}

fn release_hint(queue_name: &str, global: bool, ticket: &Ticket) -> String {
    format!("buzzer done {queue_name}{} --ticket {}", if global { " -g" } else { "" }, ticket.id)
}

/// Returns false when stopped by a signal before getting a turn.
fn wait_for_turn(queue: &Queue, ticket: &mut Ticket, owner_pid: Option<u32>, quiet: bool, hint: &str, stop: &AtomicBool) -> bool {
    let mut announced = 0;
    while !take_turn_if_first(queue, ticket, owner_pid) {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        let place = place_in_line(queue, ticket);
        if place != announced && !quiet {
            eprintln!("buzzer: waiting on {} (place {place} in line)", queue.name);
            announced = place;
        }
        sleep(POLL);
    }
    if !quiet {
        println!("buzzer: YOUR TURN on {} (ticket {}). Release with: {hint}", queue.name, ticket.id);
        let _ = std::io::stdout().flush();
    }
    true
}

fn leave(queue: &Queue, ticket: &Ticket) {
    let _guard = lock(queue);
    remove_ticket(queue, &ticket.id);
}

fn command_wait(at: Where, join_options: JoinOptions) -> i32 {
    let queue = open_queue(&at.queue, at.global);
    let stop = register_stop_flag();
    let mut ticket = join(&queue, &join_options);
    let hint = release_hint(&at.queue, at.global, &ticket);
    if wait_for_turn(&queue, &mut ticket, detect_session_pid(), join_options.quiet, &hint, &stop) {
        return 0;
    }
    leave(&queue, &ticket);
    130
}

fn command_run(at: Where, join_options: JoinOptions, command: Vec<String>) -> i32 {
    let queue = open_queue(&at.queue, at.global);
    let stop = register_stop_flag();
    let mut ticket = join(&queue, &join_options);
    let hint = release_hint(&at.queue, at.global, &ticket);
    if !wait_for_turn(&queue, &mut ticket, Some(process::id()), join_options.quiet, &hint, &stop) {
        leave(&queue, &ticket);
        return 130;
    }
    let code = run_child(&command, &stop);
    leave(&queue, &ticket);
    code
}

fn run_child(command: &[String], stop: &AtomicBool) -> i32 {
    let mut child = match Command::new(&command[0]).args(&command[1..]).spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!("buzzer: cannot run {}: {error}", command[0]);
            return 127;
        }
    };
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            use std::os::unix::process::ExitStatusExt;
            return status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(0));
        }
        if stop.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            return 130;
        }
        sleep(Duration::from_millis(50));
    }
}

fn command_done(at: Where, ticket: String) -> i32 {
    let queue = open_queue(&at.queue, at.global);
    let _guard = lock(&queue);
    reap(&queue, now_secs());
    let Some(current) = holder(&queue) else {
        println!("buzzer: nobody holds {}", queue.name);
        return 0;
    };
    if current.id != ticket {
        eprintln!(
            "buzzer: ticket {ticket} does not hold {}; only the holder can release it (use kick to override)",
            queue.name
        );
        return 1;
    }
    remove_ticket(&queue, &current.id);
    println!("buzzer: released {}", queue.name);
    0
}

fn command_renew(at: Where, hold: Option<String>) -> i32 {
    let queue = open_queue(&at.queue, at.global);
    let _guard = lock(&queue);
    let Some(mut current) = holder(&queue) else {
        eprintln!("buzzer: nobody holds {}", queue.name);
        return 1;
    };
    current.expires = now_secs() + hold.map_or(current.hold, |h| parse_duration(&h));
    write_ticket(&queue, &current);
    println!("buzzer: renewed {}", queue.name);
    0
}

fn command_kick(at: Where, ticket: String) -> i32 {
    let queue = open_queue(&at.queue, at.global);
    let _guard = lock(&queue);
    if !read_tickets(&queue).iter().any(|t| t.id == ticket) {
        eprintln!("buzzer: no ticket {ticket} on {}", queue.name);
        return 1;
    }
    remove_ticket(&queue, &ticket);
    println!("buzzer: removed ticket {ticket} from {}", queue.name);
    0
}

fn describe(ticket: &Ticket, now: u64) -> String {
    let who = if ticket.label.is_empty() { &ticket.cwd } else { &ticket.label };
    if ticket.held {
        return format!("HOLDS   {who}  (ticket {}, {}s left)", ticket.id, ticket.expires.saturating_sub(now));
    }
    format!("waiting {who}  (ticket {}{})", ticket.id, if ticket.jumped { ", jumped" } else { "" })
}

fn queue_names(global: bool) -> Vec<String> {
    let (base, _) = scope_dir(global);
    let mut names: Vec<String> = fs::read_dir(base)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

fn command_status(queue_name: Option<String>, global: bool) -> i32 {
    let names = queue_name.map_or_else(|| queue_names(global), |n| vec![n]);
    if names.is_empty() {
        println!("buzzer: no queues here");
    }
    let now = now_secs();
    for name in names {
        let queue = open_queue(&name, global);
        let (current, waiting) = {
            let _guard = lock(&queue);
            reap(&queue, now);
            (holder(&queue), line(&queue))
        };
        println!("{}", queue.name);
        if current.is_none() && waiting.is_empty() {
            println!("  (idle)");
        }
        for ticket in current.iter().chain(waiting.iter()) {
            println!("  {}", describe(ticket, now));
        }
    }
    0
}

fn main() {
    let code = match Cli::parse().verb {
        Verb::Wait { at, join } => command_wait(at, join),
        Verb::Run { at, join, command } => command_run(at, join, command),
        Verb::Done { at, ticket } => command_done(at, ticket),
        Verb::Renew { at, hold } => command_renew(at, hold),
        Verb::Kick { at, ticket } => command_kick(at, ticket),
        Verb::Status { queue, global } => command_status(queue, global),
    };
    process::exit(code);
}
