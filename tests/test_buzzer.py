import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest

BUZZER = os.path.join(os.path.dirname(__file__), "..", "buzzer")


class BuzzerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.home = os.path.join(self.temp.name, "state")
        self.repo = os.path.join(self.temp.name, "repo")
        self.worktree = os.path.join(self.temp.name, "repo-wt")
        self.other_repo = os.path.join(self.temp.name, "other")
        self.procs = []
        for folder in (self.repo, self.other_repo):
            os.makedirs(folder)
            self.git(folder, "init", "-q")
            self.git(folder, "-c", "user.email=a@b", "-c", "user.name=t", "commit", "-q", "--allow-empty", "-m", "x")
        self.git(self.repo, "worktree", "add", "-q", self.worktree)

    def tearDown(self):
        for proc in self.procs:
            if proc.poll() is None:
                proc.kill()
            proc.wait()
            proc.stdout.close()
            proc.stderr.close()
        self.temp.cleanup()

    def git(self, cwd, *args):
        subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True)

    def env(self):
        return {**os.environ, "BUZZER_HOME": self.home}

    def run_cli(self, *args, cwd=None, timeout=10):
        return subprocess.run([sys.executable, BUZZER, *args], cwd=cwd or self.repo, env=self.env(),
                              capture_output=True, text=True, timeout=timeout)

    def start(self, *args, cwd=None):
        proc = subprocess.Popen([sys.executable, BUZZER, *args], cwd=cwd or self.repo, env=self.env(),
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.procs.append(proc)
        return proc

    def waiting_count(self, queue, cwd=None, *flags):
        text = self.run_cli("status", queue, *flags, cwd=cwd).stdout
        return text.count("waiting ")

    def done(self, ticket=None, queue="device"):
        if ticket is None:
            ticket = self.run_cli("status", queue).stdout.split("HOLDS")[1].split("ticket ")[1].split(",")[0]
        return self.run_cli("done", queue, "--ticket", ticket)

    def wait_until(self, condition, seconds=5):
        end = time.time() + seconds
        while time.time() < end:
            if condition():
                return True
            time.sleep(0.05)
        return False

    def test_first_taker_gets_it_at_once(self):
        result = self.run_cli("wait", "device", "--label", "a")
        self.assertEqual(result.returncode, 0)
        self.assertIn("YOUR TURN", result.stdout)
        self.assertIn("HOLDS   a", self.run_cli("status", "device").stdout)

    def test_second_waits_until_done_then_wakes(self):
        self.run_cli("wait", "device")
        second = self.start("wait", "device", "--label", "b")
        self.assertTrue(self.wait_until(lambda: self.waiting_count("device") == 1))
        self.assertIsNone(second.poll())
        self.done()
        second.wait(timeout=5)
        self.assertEqual(second.returncode, 0)
        self.assertIn("YOUR TURN", second.stdout.read())

    def test_fifo_order(self):
        self.run_cli("wait", "device")
        waiters = []
        for label in ("b", "c", "d"):
            waiters.append(self.start("wait", "device", "--label", label))
            self.assertTrue(self.wait_until(lambda n=len(waiters): self.waiting_count("device") == n))
        for waiter in waiters:
            self.assertIsNone(waiter.poll())
            self.done()
            waiter.wait(timeout=5)

    def test_jump_goes_ahead_of_waiters_but_not_the_holder(self):
        self.run_cli("wait", "device", "--label", "holder")
        slow = self.start("wait", "device", "--label", "slow")
        self.assertTrue(self.wait_until(lambda: self.waiting_count("device") == 1))
        urgent = self.start("wait", "device", "--first", "--label", "urgent")
        self.assertTrue(self.wait_until(lambda: self.waiting_count("device") == 2))
        self.assertIn("HOLDS   holder", self.run_cli("status", "device").stdout)
        self.done()
        urgent.wait(timeout=5)
        self.assertIsNone(slow.poll())

    def test_worktrees_share_project_queue(self):
        self.run_cli("wait", "editor", cwd=self.repo)
        second = self.start("wait", "editor", cwd=self.worktree)
        self.assertTrue(self.wait_until(lambda: self.waiting_count("editor", self.repo) == 1))
        self.assertIsNone(second.poll())

    def test_other_project_has_its_own_queue(self):
        self.run_cli("wait", "editor", cwd=self.repo)
        other = self.run_cli("wait", "editor", cwd=self.other_repo, timeout=3)
        self.assertEqual(other.returncode, 0)

    def test_global_scope_is_shared_across_projects_and_separate_from_project(self):
        self.run_cli("wait", "phone", "-g", cwd=self.repo)
        blocked = self.start("wait", "phone", "-g", cwd=self.other_repo)
        self.assertTrue(self.wait_until(lambda: self.waiting_count("phone", self.repo, "-g") == 1))
        self.assertIsNone(blocked.poll())
        same_name_project = self.run_cli("wait", "phone", cwd=self.repo, timeout=3)
        self.assertEqual(same_name_project.returncode, 0)

    def test_waiter_that_dies_loses_its_place(self):
        self.run_cli("wait", "device")
        doomed = self.start("wait", "device", "--label", "doomed")
        self.assertTrue(self.wait_until(lambda: self.waiting_count("device") == 1))
        doomed.kill()
        doomed.wait()
        survivor = self.start("wait", "device", "--label", "survivor")
        self.assertTrue(self.wait_until(lambda: "survivor" in self.run_cli("status", "device").stdout))
        self.done()
        survivor.wait(timeout=5)
        self.assertEqual(survivor.returncode, 0)

    def test_terminated_waiter_cleans_its_ticket(self):
        self.run_cli("wait", "device")
        waiter = self.start("wait", "device")
        self.assertTrue(self.wait_until(lambda: self.waiting_count("device") == 1))
        waiter.send_signal(signal.SIGTERM)
        waiter.wait(timeout=5)
        self.assertEqual(self.waiting_count("device"), 0)

    def test_run_releases_when_command_ends(self):
        result = self.run_cli("run", "device", "--", sys.executable, "-c", "print('hi')")
        self.assertEqual(result.returncode, 0)
        self.assertIn("hi", result.stdout)
        self.assertIn("(idle)", self.run_cli("status", "device").stdout)

    def test_run_passes_exit_code_through(self):
        result = self.run_cli("run", "device", "--", sys.executable, "-c", "raise SystemExit(7)")
        self.assertEqual(result.returncode, 7)
        self.assertIn("(idle)", self.run_cli("status", "device").stdout)

    def test_run_holder_killed_frees_queue(self):
        holder = self.start("run", "device", "--", sys.executable, "-c", "import time; time.sleep(60)")
        self.assertTrue(self.wait_until(lambda: "HOLDS" in self.run_cli("status", "device").stdout))
        holder.kill()
        holder.wait()
        again = self.run_cli("wait", "device", timeout=5)
        self.assertEqual(again.returncode, 0)

    def test_hold_limit_frees_a_forgotten_hold(self):
        self.run_cli("wait", "device", "--hold", "1s")
        time.sleep(1.3)
        again = self.run_cli("wait", "device", timeout=5)
        self.assertEqual(again.returncode, 0)

    def test_renew_extends_hold(self):
        self.run_cli("wait", "device", "--hold", "1s")
        self.assertEqual(self.run_cli("renew", "device", "--hold", "1h").returncode, 0)
        time.sleep(1.3)
        self.assertIn("HOLDS", self.run_cli("status", "device").stdout)

    def test_kick_removes_a_ticket(self):
        self.run_cli("wait", "device")
        status = self.run_cli("status", "device").stdout
        ticket = status.split("ticket ")[1].split(",")[0].split(")")[0]
        self.assertEqual(self.run_cli("kick", "device", "--ticket", ticket).returncode, 0)
        self.assertIn("(idle)", self.run_cli("status", "device").stdout)

    def test_done_with_wrong_ticket_is_refused(self):
        self.run_cli("wait", "device")
        refused = self.done(ticket="nope")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("HOLDS", self.run_cli("status", "device").stdout)
        self.assertNotEqual(self.run_cli("done", "device").returncode, 0)

    def test_bad_queue_name_is_refused(self):
        self.assertNotEqual(self.run_cli("wait", "../evil").returncode, 0)


if __name__ == "__main__":
    unittest.main()
