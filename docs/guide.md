# buzzer guide

For any agent, script or person. Nothing here assumes code, a language or a tool.
Print it any time with `buzzer guide`.

## The idea

Some things can only be used by one party at a time: a printer, a staging server, a shared document
being rewritten, a test phone, a meeting room booking, a licence seat. A **queue** is a line for one such thing.
You wait your turn, use the thing, and hand it on. The queue is named after the **thing**, never the task.

`buzzer wait NAME` is a normal command that **finishes when it is your turn**. That is the whole trick:
anything that can run a command and notice when it ends can queue. No messages, no polling, no server.

## The five commands that matter

```bash
buzzer run NAME -- COMMAND ...      # wait, run the command, give the turn back when it ends (use this first)
buzzer wait NAME                    # wait; finishes when it is your turn; you hold it until you say done
buzzer done NAME --ticket ID        # give the turn back (the ID is printed when your turn starts)
buzzer status [NAME]                # who holds it, for how long, who is waiting
buzzer guards                       # the shared things this project has listed, with their rules
```

## Choosing between `run` and `wait`

- **`run`** when one command does the whole job. The turn ends with the command, even if it fails or is killed. Nothing to forget.
- **`wait`** when the job has several steps you cannot put in one command. Start it **in the background** and carry on
  with other work; when the command finishes, it is your turn. Never sit and watch it. Write down the ticket ID
  it prints. Give the turn back with `done` when you stop using the thing, even if the work failed.

## Manners (the part that matters most)

1. **Check the list first.** Run `buzzer guards`. If the thing you are about to use is listed, queue for it. If you are unsure whether
   something is shared, ask or check `buzzer status`; do not guess.
2. **Take the turn only when you are ready to use the thing right now.** Finish the build, the download, the draft, the plan first.
   Never take the turn and then prepare while holding it.
3. **Give it back the moment you stop using it.** Do not hold across a wait (a build, a download, an answer from a person, another job).
   Give it back and queue again for the next step.
4. **Never hold "just in case".** A turn you are not using is time stolen from whoever is waiting.
5. **One step, one turn.** Keep turns short. A turn frees itself after 30 minutes unless you extend it
   (`buzzer renew NAME --hold 1h`), so extend only when the step truly needs it.
6. **Do not jump the line** (`--first`) unless a person with the authority told you to. Even then you never take the turn from whoever holds it now.
7. **Do not remove someone else's ticket** (`kick`). A turn whose owner died frees itself. If a live holder looks stuck,
   run `buzzer status`, tell a person who holds it and for how long, and ask.
8. **Use the shared thing only through the queue.** If someone else might touch it, queue first, every time.

## Naming

- Name the thing: `staging-server`, `label-printer`, `brand-deck`, `test-phone-1`. Not `my-test` or `friday-release`.
- Letters, digits, `.`, `_`, `-`. Same thing, same name, for everyone.
- Before inventing a name, run `buzzer guards` and `buzzer status`; the thing may already have one.

## Scopes: whose list is it?

- **Project (default):** shared by everyone working in the same git project, including all its worktrees. Use for things that belong to the project.
- **Global (`-g`):** shared by everything on this machine. Use for things every project shares (the phone on the desk).
- The same name in two scopes is two different queues.

## The list of shared things

Say what is shared and the rule for it, in plain words, once per project:

```bash
buzzer guard staging-server "Only one person deploys at a time. Queue before you deploy."
buzzer guard brand-deck "One editor at a time, or edits overwrite each other."
buzzer guard test-phone -g "The shared phone on the desk. Queue before you install or run anything on it."
buzzer guards          # print the list (project first, then global)
buzzer unguard NAME    # remove an entry
```

A rule is only text for a reader. buzzer does not interpret it.
To show the list at the start of a job, run `buzzer guards --quiet-if-empty` from whatever starts your agents or scripts (it prints nothing when the list is empty).

## When something goes wrong

| What you see | What to do |
|---|---|
| `waiting ... (place N in line)` | Normal. Carry on with other work; you finish waiting when your command ends. |
| You waited far too long | `buzzer status NAME`. If the holder has held it a long time, tell a person. Do not kick. |
| `ticket ... does not hold` | That ticket is not the current turn. Check `buzzer status NAME`; your turn may have timed out. |
| Your turn ended on its own | The 30-minute limit passed, or your session died. Queue again; do not assume you still have it. |
| `buzzer` is not installed | `brew tap alon-talmi/buzzer https://github.com/alon-talmi/buzzer`, `brew trust alon-talmi/buzzer`, `brew install buzzer` (a person runs the trust step). |

## Keeping a turn alive only while its owner lives

With `wait`, buzzer cannot know who you are. Pass the id of the process that should keep the turn alive:
`buzzer wait NAME --owner PID` (or set `BUZZER_OWNER_PID`). The turn ends when that process ends, so a crashed agent or job
never blocks the line. Without it, only the 30-minute limit frees a forgotten turn. `run` always follows its own command.

## Limits

One machine. Cooperative: it cannot stop something that never asks. Apple-silicon macOS builds only for now.
