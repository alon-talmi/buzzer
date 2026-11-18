# buzzer

Wait your turn for a shared thing (a test device, the Unity Editor, a port), then get buzzed.
A single small Rust binary, no server. macOS and Linux. Any tool or agent can use it.

## Install

```bash
brew tap alon-talmi/buzzer https://github.com/alon-talmi/buzzer   # once
brew install buzzer
brew update && brew upgrade buzzer                                  # later
# or from source: cargo install --path .
```

## Why no messages and no polling

`buzzer wait NAME` is a normal command that **exits when it is your turn**.
Run it in the background of an agent session (Claude Code: Bash `run_in_background`) and the
harness wakes the session when the command exits. Nobody messages anybody.

## Use

```bash
buzzer wait editor                  # block until it is my turn, then exit 0 (I now hold it)
buzzer done editor --ticket ID      # I am finished (ID printed when my turn came; only the holder can release)
buzzer run editor -- npm test       # wait, run, release when the command ends (safest)
buzzer wait editor --first          # jump ahead of everyone waiting (never kicks the holder out)
buzzer status                       # every queue in this project
buzzer status -g                    # every global queue
buzzer renew editor --hold 1h       # still working, restart the hold timer
buzzer kick editor --ticket ID      # remove a stuck ticket (ID is in `status`)
```

Options for `wait` and `run`: `--label "who I am"` (shown in status), `--hold 2h` (default 2h),
`-q` (quiet), `-g` (global scope).

## Scopes

- **Project (default):** one queue namespace per git repository. **Every worktree of the same repo shares it.**
  Outside git it falls back to the current folder. Override with `BUZZER_PROJECT=/some/path`.
- **Global (`-g`):** one namespace for the whole machine, for things like a phone on your desk.
- The same name in different scopes is a different queue (`editor` in project A, in project B and `-g` never clash).

State lives in `~/.local/state/buzzer/` (override with `BUZZER_HOME`).

## When a hold is released by itself

| Case | What frees it |
|---|---|
| `run`: command ends or is killed | its own process, checked by process id |
| `wait`: you forget `done` | the `--hold` limit (default 2h); `renew` restarts it |
| A waiter's session or command dies | its ticket is dropped, it loses its place |

## Rules of the line

First come, first served. `--first` goes before everyone waiting, but never before the current holder.
Several `--first` callers go in the order they asked.

## Limits

One machine only. Claims are cooperative: a process that never calls buzzer is not stopped.
`wait` cannot see if the holder's session died (only `run` can), so it relies on the hold limit.

## Tests

```bash
cargo test
```
