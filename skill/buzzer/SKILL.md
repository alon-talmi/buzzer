---
name: buzzer
description: Take turns on a shared thing with other sessions using the `buzzer` CLI. Use before running anything that only one session may use at a time (a connected test device, the Unity Editor, automation tests, a port), or when asked to queue, wait your turn, or check who holds something.
---

# buzzer

`buzzer` is a queue shared by every session and worktree on this machine. You wait your turn and the command exits when it is yours. Nobody messages anybody.

## Default: `run`

```bash
buzzer run <queue> --label "<what I am doing>" -- <command>
```

Waits, runs the command, releases when it ends or dies. Use this whenever one command does the whole job. It cannot be forgotten.

## When you need several steps: `wait` in the background

1. Start `buzzer wait <queue> --label "<what I am doing>"` with Bash `run_in_background`. Do not poll it. You are woken when it exits (your turn).
2. The output holds the line `Release with: buzzer done <queue> --ticket <ID>`. Keep that ID.
3. Do the work. Long job? `buzzer renew <queue> --hold 3h` (a hold frees itself after 30 minutes unless renewed).
4. Release: `buzzer done <queue> --ticket <ID>`. Always, even if the work failed.

## Scopes

- Default is this project's queues, shared by all of its worktrees. Names like `editor`, `device`, `appium`.
- `-g` is machine-wide, for things every project shares (a phone on the desk). Same name in each scope is a different queue.

## Do not hog it

The queue name is the **thing being shared** (a phone, a port, the Editor), never a task. Anyone waiting is blocked for as long as you hold it.

- **Take the turn only when you are ready to use the thing right now:** build finished, files downloaded, script written, tests chosen. Never take it first and prepare while holding.
- **Release the moment you stop using it.** Do not hold across a build, a download, a wait on another job or on the user. Release, and queue again for the next phase.
- **Never hold "just in case".** A hold you are not using is a bug, even when you are about to use it again.
- **One phase = one turn.** `buzzer run` makes that automatic: it ends with the command.
- A hold frees itself after 30 minutes. If a phase truly needs longer, `buzzer renew <queue> --hold 1h`, and check `buzzer status` to see how long you have held.

## Rules

- `--first` jumps the line. Only when the user told you to. It never takes the turn from the current holder.
- `buzzer status` shows who holds each queue, for how long and when it was last renewed; `buzzer status -g` shows the global ones.
- A turn whose session died frees itself, so you rarely need `kick`. Never `kick` a live ticket that is not yours: tell the user who holds it and for how long, and ask.
- Use the shared thing only through the queue. If another session may touch it, queue first.
- If `buzzer` is missing: `brew tap alon-talmi/buzzer https://github.com/alon-talmi/buzzer && brew trust alon-talmi/buzzer && brew install buzzer` (ask the user to run the trust step).
