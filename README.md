# buzzer

Wait your turn for a shared thing (a test device, the Unity Editor, a port), then get buzzed.
A single small Rust binary, no server. Apple-silicon macOS only. Any tool or agent can use it.

## Install

```bash
brew tap alon-talmi/buzzer https://github.com/alon-talmi/buzzer   # once
brew trust alon-talmi/buzzer                                        # newer brew asks for this
brew install buzzer
brew update && brew upgrade buzzer                                  # later
# or from source: cargo install --path .
```

## Why no messages and no polling

`buzzer wait NAME` is a normal command that **exits when it is your turn**.
Run it in the background of an agent session (for example a background shell command) and the
harness wakes the session when the command exits. Nobody messages anybody.

Full instructions, update, uninstall and troubleshooting: [docs/install.md](docs/install.md).

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

Options for `wait` and `run`: `--label "who I am"` (shown in status), `--hold 30m` (default 30m),
`--owner PID` (turn ends when that process ends), `-q` (quiet), `-g` (global scope).

## Do not hog it

The queue name is the thing being shared (a phone, a port, the Editor). Take the turn only when you are
ready to use it right now, release the moment you stop (do not hold across a build, a download or a wait),
and queue again for the next phase. Never hold "just in case". A hold frees itself after 30 minutes unless renewed.

## Guide for agents and people

`buzzer guide` prints [docs/guide.md](docs/guide.md): the idea, manners, naming, scopes, what to do when something goes wrong.
Point your agents at it (a line in their instructions: "shared things go through buzzer; run `buzzer guide`").
List what is shared in a project with `buzzer guard NAME "rule in plain words"`, and print it with `buzzer guards`.

An example skill for one specific agent tool lives in `examples/claude-skill/` (optional, not part of buzzer).

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
| `wait --owner PID`: that process ends | the turn is released |
| `wait`: you forget `done` | the `--hold` limit (default 30m); `renew` restarts it |
| A waiter's session or command dies | its ticket is dropped, it loses its place |

## Rules of the line

First come, first served. `--first` goes before everyone waiting, but never before the current holder.
Several `--first` callers go in the order they asked.

## Limits

One machine only. Claims are cooperative: a process that never calls buzzer is not stopped.
Without `--owner`, `wait` only frees itself at the hold limit.

## Tests

```bash
cargo test
```
