# Installing buzzer

## Requirements

- A Mac with Apple silicon (M1 or newer). Intel Macs, Linux and Windows are not supported.
- [Homebrew](https://brew.sh).
- `git` on your PATH (project queues use it to find the project; it is already there on any developer Mac).

## Install

```bash
brew tap alon-talmi/buzzer https://github.com/alon-talmi/buzzer
brew trust alon-talmi/buzzer
brew install buzzer
```

- The `tap` line is needed once, because the repository is not named `homebrew-…`.
- `brew trust` is needed once. Newer Homebrew refuses to load a formula from a tap you have not trusted.
- The install downloads a small prebuilt binary (about 300 KB). It does not need Rust or a compiler.

Check it worked:

```bash
buzzer --version
buzzer guide        # the guide for agents and people
```

## Update

```bash
brew update && brew upgrade buzzer
```

Each release is a git tag (`v0.1.7`, …). The release workflow builds the binary, publishes it and updates the formula, so `brew upgrade` is all anyone needs.

## Uninstall

```bash
brew uninstall buzzer
brew untap alon-talmi/buzzer
rm -rf ~/.local/state/buzzer     # optional: forget every queue and guard list
```

## Build from source instead

```bash
git clone https://github.com/alon-talmi/buzzer
cd buzzer
cargo install --path .           # installs to ~/.cargo/bin
cargo test                       # optional
```

## Set it up for a project

1. List what is shared, once, in the project's folder:
   ```bash
   buzzer guard staging-server "Only one person deploys at a time. Queue before you deploy."
   buzzer guard test-phone -g "The shared phone on the desk. Queue before you use it."   # -g = every project on this machine
   buzzer guards
   ```
2. Tell your agents (a line in their instructions, or the start of a job script):
   > Shared things go through `buzzer`. Run `buzzer guards` to see what is shared and `buzzer guide` for how to queue.
   > Not installed: `brew tap alon-talmi/buzzer https://github.com/alon-talmi/buzzer`, then `brew install buzzer`.
3. Use it: `buzzer run staging-server -- ./deploy.sh`.

The guard list and the queues live on each machine (`~/.local/state/buzzer/`, or `$BUZZER_HOME`).
Every person who should be protected needs buzzer installed and the same guards listed on their machine.
Every worktree of one git project on a machine shares that project's queues.

## Troubleshooting

| Problem | Fix |
|---|---|
| `Refusing to load formula … from untrusted tap` | `brew trust alon-talmi/buzzer` |
| `buzzer: command not found` after install | Open a new terminal; check `brew --prefix` is on your PATH. |
| `buzzer --version` shows an old version | Another copy comes earlier in PATH. Run `which -a buzzer` and remove the old one. |
| Brew says the formula needs an arm64 Mac | You are on an Intel Mac. Build from source (above). |
| Queues look empty on a second machine | State is per machine. Install and list guards there too. |
