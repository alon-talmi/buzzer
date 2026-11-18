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
3. Do the work. Long job? `buzzer renew <queue> --hold 3h` (the hold frees itself after 2h by default).
4. Release: `buzzer done <queue> --ticket <ID>`. Always, even if the work failed.

## Scopes

- Default is this project's queues, shared by all of its worktrees. Names like `editor`, `device`, `appium`.
- `-g` is machine-wide, for things every project shares (a phone on the desk). Same name in each scope is a different queue.

## Rules

- Queue name = the thing, not the task: `editor`, `device`, not `my-test`.
- `--first` jumps the line. Only when the user told you to. It never takes the turn from the current holder.
- Never `kick` a ticket that is not yours. If a holder looks stuck, run `buzzer status`, tell the user who holds it and for how long, and ask.
- `buzzer status` shows every queue here, `buzzer status -g` the global ones.
- If `buzzer` is missing: `brew tap alon-talmi/buzzer https://github.com/alon-talmi/buzzer && brew trust alon-talmi/buzzer && brew install buzzer` (ask the user to run the trust step).
