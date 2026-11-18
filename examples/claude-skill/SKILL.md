---
name: buzzer
description: Take turns on a shared thing with other sessions using the `buzzer` CLI. Use before touching anything only one party may use at a time, or when asked to queue, wait your turn, or check who holds something.
---

Run `buzzer guide` and follow it. Run `buzzer guards` to see what this project lists as shared.

Quick form: `buzzer run <thing> -- <command>`. For several steps, run `buzzer wait <thing> --owner $PPID` as a background command
(`$PPID` is this session's process, so the turn ends if the session dies), keep the ticket ID it prints, and release with
`buzzer done <thing> --ticket <ID>` the moment you stop using the thing. Take the turn only when ready to use it right now.
