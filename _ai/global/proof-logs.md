# Logging

## Every line is a proof

In this project logs are **not** throwaway debug output kept only for
development. In production too, every line is a **proof log**: evidence that a
business flow actually happened, in the order and shape the spec says it should.
There is no separate "debug logging" vs "prod logging" — the same lines serve
both, and they stay on in production.

A **proof line** (or just "a line") is one proof log pinned to three things:

* a specific **session** — *who* was interacting (the browser session);
* a specific **request** — *which* call, traced across every service it touched;
* a specific **feature** (the **stream**) — *what* business flow it belongs to.

Those are exactly the three fields the `follower` trailer carries
(`[[session][request][feature]]`), which is why every line can be attributed
unambiguously.

**The ultimate goal — reinjection.** The integration tests are the source of
truth for what a feature *should* log. Running the tests for a stream produces a
reference set of proof lines — the "proven" trace for that feature. Later, when
something goes wrong in production, we compare the prod lines for that stream
against the IT proof lines (**reinjection of lines**) and see exactly where the
real run diverged from the proven one. So the proof logs are a debugging and
regression tool, not just a record.

## What to log

Logging is **mandatory** in `fisher-server`. Emit a `log info` (or appropriate
level):

* every time a service **starts** and **ends**;
* every time a meaningful action is **completed**;
* every time a meaningful action has **failed**;
* every time a process is **routed onto an important path** (a significant branch).

Guidelines:

* Include the `uuid` of the current game in log lines so a game can be traced
  end to end.
* **Only `INFO`, `WARN` and `ERROR` are meaningful.** We do not use logs to
  debug, so there is no `DEBUG` or `TRACE` level here — a line either proves
  something worth keeping in production or it should not exist. Use the three
  levels deliberately:
  * `INFO` — the proof events above (entry/exit, decisions, milestones, …);
  * `WARN` — a recoverable problem the flow handled and continued past;
  * `ERROR` — a failure that aborts the action.
* Log facts, not secrets — never log full request bodies that could contain
  sensitive data.

## Stream codes — one tag, two jobs

Every log line carries a short **stream** tag. In this project the stream is the
**feature name** it belongs to — the same feature the specs and tests use. Today
we have exactly one feature, **START-A-GAME**, so `START-A-GAME` is the only
stream; each new feature adds its own. One token does two jobs at once:

1. **A label** you can `grep` for — one business flow. `grep "START-A-GAME"`
   gives you every line for *start a game*, across handlers and helpers.
2. **An on/off switch** — a stream can be muted or enabled without touching the
   code, so in production you can focus on one flow and drop the noise of the
   rest.

That is the whole point: you don't keep a separate "log tag" convention and a
separate "log level per module" config — a single value is both.

Keep the set of streams **closed and discoverable** (one enum / one list), so a
code can't be misspelled into existence and every stream is visible in one place.

### How the switch behaves

The enabled streams are read once, at boot, from a config list. Three states
worth memorising:

* **List absent / empty** → every stream is on (the default; nothing is hidden).
* **Non-empty list** → only the listed codes are emitted; the rest are silently
  dropped.

To zoom in on one flow in production, keep only its stream — e.g.
`["START-A-GAME"]` — and restart. No code change, no redeploy. A disabled stream
should short-circuit *before* the message is formatted, so muted flows cost
almost nothing.

## Anatomy of a line

```
2026-07-04T09:12:03Z  INFO  [fisher_server]  [fisher-server/src/game/start.rs:40]  start-game served uuid=1f0c… follower=[[279bac825ca7][424557][START-A-GAME]]
```

| part                                              | source                                                     |
| ------------------------------------------------- | ---------------------------------------------------------- |
| `2026-07-04T09:12:03Z`                            | timestamp, from the subscriber                             |
| `INFO`                                            | level                                                      |
| `[fisher_server]`                                 | the tracing **target** — the crate/**module** that emitted |
| `[fisher-server/src/game/start.rs:40]`            | file:line                                                  |
| message + `uuid=…`                                 | yours; carry the game `uuid`                               |
| `follower=[[279bac825ca7][424557][START-A-GAME]]` | the **trace context**, appended to every line (see below)  |

The `follower` trailer carries three fields, in order:

| field          | meaning                                                                    |
| -------------- | -------------------------------------------------------------------------- |
| `279bac825ca7` | **session id** — the browser session the request belongs to               |
| `424557`       | **request tracking id** — one request, passed through the different services (front → `fisher-server` → Stockfish) |
| `START-A-GAME` | the **stream** — which business flow / feature you tagged the line with    |

Two concepts that are easy to mix up:

* **module** = the `[fisher_server]` part — *which crate/module* `tracing` says
  emitted the line. Automatic.
* **stream** = the last `follower` field (`START-A-GAME`) — *which business flow*
  you tagged it with. Yours, and the thing you toggle.

> Current status: `fisher-server` logs via `tracing` today, and `main.rs` builds
> the subscriber with `.with_target(false)`, so the `[fisher_server]` module part
> is not printed yet — enable the target to reach the full line shown here. The
> `follower` trailer (session id, request tracking id, stream) and the
> enable/disable list are the **target design** for this project, not yet
> implemented.

## Tracing — the `follower` keys

The `follower` trailer gives you three keys to slice the logs, plus the game
`uuid` in the message itself:

* `grep "279bac825ca7"` — everything in **one browser session**.
* `grep "424557"` — **one request** end to end, across every service it touched
  (front → `fisher-server` → Stockfish); the request tracking id is stable as it
  is passed through.
* `grep "START-A-GAME"` — scopes to **one feature** (stream).
* `grep "<uuid>"` — a whole **game**, since every line carries its `uuid`.

Combine keys to isolate a single flow — e.g. one request within one session.

## Proof logs

Examples are drawn from the one feature we have — **START-A-GAME**.

| # | Placement Rule                         | Purpose                                                                       | Placement                                                                                                        | START-A-GAME example                             |
| - | -------------------------------------- | ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| 1 | **Feature Entry**                      | Prove that a business routine started.                                        | **Before** the routine starts.                                                                                  | `START-A-GAME.start` — start-game requested      |
| 2 | **Feature Exit**                       | Prove that the business routine completed (success or failure).               | **After** the routine completes (or in a `finally` block).                                                      | `START-A-GAME.end`, `result=SUCCESS`, `uuid=1f0c…` |
| 3 | **External Boundary (Before & After)** | Prove every interaction with an external system.                              | **Request:** before sending the request. **Response:** after receiving the response.                            | _none in START-A-GAME_ (no Stockfish call to start a game) |
| 4 | **Business Decision**                  | Prove which business path was chosen and why.                                 | **Immediately after** the decision has been made.                                                               | `mode=random`                                    |
| 5 | **State Change**                       | Prove every persistent business state transition.                             | **Immediately after** the state has actually changed (or after the transaction commits if that's what matters). | `game created uuid=1f0c…` (registered)           |
| 6 | **Invariant / Business Rule Check**    | Prove that an important rule was evaluated and its result.                    | **Immediately after** the check has been evaluated.                                                             | `piece_count_valid=true`                         |
| 7 | **Business Milestone**                 | Prove completion of a major logical step.                                     | **After** the milestone has successfully completed.                                                             | `board_generated`                                |
| 8 | **Error / Exception**                  | Prove where the routine failed, with enough context to reconstruct execution. | **At the point** where the error is detected or handled.                                                        | `invalid mode`, `invalid piece count`            |
