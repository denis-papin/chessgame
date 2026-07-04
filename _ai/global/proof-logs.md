# Proof of execution

## Why AIAD needs proof

This project is built with **AIAD** (AI-Aided Development): the specs under
`_ai/` say what each feature must do, an AI writes the code from those specs,
and the tests decide whether the result is accepted. In that workflow no human
reads all the generated code — so trust cannot come from code review. It has to
come from **evidence**.

That evidence has two halves:

1. **Proof of behaviour** — the integration tests. They drive the feature
   through its public APIs and prove, at build time, that the code satisfies
   the spec.
2. **Proof of execution** — the logs. They prove, at run time (including
   production), that a business flow actually happened, in the order and shape
   the spec says it should.

This document defines the second half. In this project logs are **not**
throwaway debug output kept only for development. Every **log** is a **proof
log** — one piece of execution evidence, kept on in production. There is no
separate "debug logging" vs "prod logging"; the same logs serve both. Grouped
by execution context, those proof logs form **proof lines** — the central
concept of proof of execution, defined next.

## What a proof line is — the unit of proof

Three terms build on each other, and it matters not to confuse them:

* A **log** (a log entry) is *one* line of text in the output: a single event at
  one point in the code, carrying a timestamp, a level, the emitting module and
  `file:line`, a business message (with the game `uuid`), and the `follower`
  trailer. See [Anatomy of a log](#anatomy-of-a-log).

* A **proof line** is the **ordered list of logs produced for one execution
  context**, from the start of a call to its end. It is *not* a single log — it
  is the whole run of them for that context. A proof line **is the execution
  path**: read in order, its logs trace exactly which route the code took
  through the feature — every branch, decision, and step it actually went
  through. This is the key concept of proof of execution: a proof line is a
  complete, self-contained record of *one execution of one feature*, and it is
  the thing we prove, compare, and reinject.

* A **set of proof lines** is **all the proof lines for one feature (stream)** —
  every execution path of that feature, produced by running its **integration
  tests**. The IT drives each scenario of the feature (success, each business
  decision, each error), and every scenario yields one proof line; together they
  are the feature's set of proof lines — its complete proven reference.

The execution context that defines a proof line is the triple carried by the
`follower` trailer — all the logs sharing this triple, in emission order, are
one proof line:

* a specific **session** — *who* was interacting (the browser session);
* a specific **request** — *which* call (the tracking id), traced across every
  service it touched;
* a specific **feature** (the **stream**) — *what* business flow it belongs to.

The integration tests produce the feature's set of proof lines — the **proven**
reference; production emits the real proof line of each execution; reinjection
matches a production proof line to its proven counterpart and compares the two.

## The proof chain: spec → test → set of proof lines → reinjection

The pieces connect into one chain, and each link uses the same key — the
**feature** name (e.g. `START-A-GAME`):

1. The **spec** (`_ai/features/Fxxxx…`) defines the feature and its rules.
2. The **integration tests** are the source of truth for what the feature
   *should* log. Running the tests for a feature produces its **set of proof
   lines** — the **proven** reference: one proof line per scenario of that
   feature.
3. In **production**, the same code emits the same logs, forming the real proof
   line for each execution of that feature.
4. When something goes wrong, we match the production proof line to its
   counterpart in the proven set and compare them — the **reinjection of proof
   lines** — and see exactly where the real run diverged from the proven one.

Reinjection is the ultimate goal: it turns the logs from a record into a
debugging and regression tool. An AIAD project can then answer "did production
do what the spec says?" by diffing two proof lines, without anyone stepping
through the code.

## What to log

Logging is **mandatory** in `fisher-server`. Emit a `log info` (or appropriate
level):

* every time a service **starts** and **ends**;
* every time a meaningful action is **completed**;
* every time a meaningful action has **failed**;
* every time a process is **routed onto an important path** (a significant branch).

Guidelines:

* Include the `uuid` of the current game in every log so a game can be traced
  end to end.
* **Only `INFO`, `WARN` and `ERROR` are meaningful.** We do not use logs to
  debug, so there is no `DEBUG` or `TRACE` level here — a log either proves
  something worth keeping in production or it should not exist. Use the three
  levels deliberately:
  * `INFO` — the proof events above (entry/exit, decisions, milestones, …);
  * `WARN` — a recoverable problem the flow handled and continued past;
  * `ERROR` — a failure that aborts the action.
* Log facts, not secrets — never log full request bodies that could contain
  sensitive data.

## Stream boundary markers — the entry/exit emoji

The two logs that bound a stream's proof line carry a fixed **emoji marker** so
the start and end of one execution are visible at a glance and trivially
`grep`-able:

| Boundary                    | Emoji | Meaning                              |
| --------------------------- | ----- | ------------------------------------ |
| **Feature Entry** (rule 1)  | 🚀    | the stream started (launch)          |
| **Feature Exit** (rule 2)   | 🏁    | the stream finished (checkered flag) |

Rules:

* The marker goes **only** on the Feature Entry and Feature Exit logs — the two
  ends of the proof line. The intermediate proof logs (decisions, invariants,
  milestones, state changes, errors) carry **no** boundary emoji, so the two
  markers unambiguously delimit one execution.
* The **exit marker rides on the exit whichever way the routine ends** — success
  *or* failure. When a run aborts on an error and its final log is the error/exit,
  that log carries 🏁 (and its `result` still records the failure).
* Put the emoji at the **start of the log message** (e.g. `🚀 start-game
  requested`, `🏁 start-game served`), before the message text and the fields.
* `grep "🚀"` lists every proof line's start; `grep "🏁"` every proof line's end
  — a fast way to count executions or spot a start with no matching finish.

## Stream codes — the feature tag

Every log carries a short **stream** tag. In this project the stream is the
**feature name** it belongs to — the same feature the specs and tests use, which
is what keeps the proof chain connected end to end. Today we have exactly one
feature, **START-A-GAME**, so `START-A-GAME` is the only stream; each new
feature adds its own.

The stream is **a label** you can `grep` for — one business flow.
`grep "START-A-GAME"` gives you every log for *start a game*, across handlers
and helpers. It is one of the three keys used to reconstruct a proof line
(below): the stream selects the logs of one feature, in both the IT run and the
prod run.

Keep the set of streams **closed and discoverable** (one enum / one list), so a
code can't be misspelled into existence and every stream is visible in one place.

## Anatomy of a log

```
2026-07-04T09:12:03Z  INFO  [fisher_server]  [fisher-server/src/game/start.rs:40]  🏁 start-game served uuid=1f0c… result=SUCCESS follower=[[279bac825ca7][424557][START-A-GAME]]
```

| part                                              | source                                                     |
| ------------------------------------------------- | ---------------------------------------------------------- |
| `2026-07-04T09:12:03Z`                            | timestamp, from the subscriber                             |
| `INFO`                                            | level                                                      |
| `[fisher_server]`                                 | the tracing **target** — the crate/**module** that emitted |
| `[fisher-server/src/game/start.rs:40]`            | file:line                                                  |
| `🏁`                                               | boundary marker — 🚀 on Feature Entry, 🏁 on Feature Exit; absent on all other logs (see [Stream boundary markers](#stream-boundary-markers--the-entryexit-emoji)) |
| message + `uuid=…`                                 | yours; carry the game `uuid`                               |
| `follower=[[279bac825ca7][424557][START-A-GAME]]` | the **trace context**, appended to every log (see below)   |

The `follower` trailer carries the three fields that place the log into a proof
line, in order:

| field          | meaning                                                                    |
| -------------- | -------------------------------------------------------------------------- |
| `279bac825ca7` | **session id** — the browser session the request belongs to               |
| `424557`       | **request tracking id** — one request, passed through the different services (front → `fisher-server` → Stockfish) |
| `START-A-GAME` | the **stream** — which business flow / feature the log belongs to          |

Two concepts that are easy to mix up:

* **module** = the `[fisher_server]` part — *which crate/module* `tracing` says
  emitted the log. Automatic.
* **stream** = the last `follower` field (`START-A-GAME`) — *which business flow*
  the log belongs to. Yours.

> Current status: `fisher-server` logs via `tracing` today, and `main.rs` builds
> the subscriber with `.with_target(false)`, so the `[fisher_server]` module part
> is not printed yet — enable the target to reach the full log shown here. The
> `follower` trailer (session id, request tracking id, stream) is the **target
> design** for this project, not yet implemented.

## Reconstructing a proof line — the `follower` keys

A proof line is reconstructed by collecting all the logs that share one
execution context. The `follower` trailer plus the game `uuid` give you the keys:

* `grep "279bac825ca7"` — every log in **one browser session**.
* `grep "424557"` — **one request** end to end, across every service it touched
  (front → `fisher-server` → Stockfish); the request tracking id is stable as it
  is passed through.
* `grep "START-A-GAME"` — the logs of **one feature** (stream).
* `grep "<uuid>"` — a whole **game**, since every log carries its `uuid`.

Combine session + request + feature to isolate exactly **one proof line** — one
execution of one feature, from start to end. That reconstructed proof line is
what reinjection compares against the proven one.

## Placement rules — where the logs go

For a proof line to be comparable against the proven one, its logs must sit at
the same, predictable points in every feature. Eight placement rules define
those points; examples are drawn from the one feature we have — **START-A-GAME**.

| # | Placement Rule                         | Purpose                                                                       | Placement                                                                                                        | START-A-GAME example                             |
| - | -------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| 1 | **Feature Entry**                      | Prove that a business routine started.                                        | **Before** the routine starts. Carries the **🚀** boundary marker.                                             | `🚀 start-game requested`                        |
| 2 | **Feature Exit**                       | Prove that the business routine completed (success or failure).               | **After** the routine completes (or in a `finally` block). Carries the **🏁** boundary marker.                 | `🏁 start-game served`, `result=SUCCESS`, `uuid=1f0c…` |
| 3 | **External Boundary (Before & After)** | Prove every interaction with an external system.                              | **Request:** before sending the request. **Response:** after receiving the response.                            | _none in START-A-GAME_ (no Stockfish call to start a game) |
| 4 | **Business Decision**                  | Prove which business path was chosen and why.                                 | **Immediately after** the decision has been made.                                                               | `mode=random`                                    |
| 5 | **State Change**                       | Prove every persistent business state transition.                             | **Immediately after** the state has actually changed (or after the transaction commits if that's what matters). | `game created uuid=1f0c…` (registered)           |
| 6 | **Invariant / Business Rule Check**    | Prove that an important rule was evaluated and its result.                    | **Immediately after** the check has been evaluated.                                                              | `piece_count_valid=true`                         |
| 7 | **Business Milestone**                 | Prove completion of a major logical step.                                     | **After** the milestone has successfully completed.                                                             | `board_generated`                                |
| 8 | **Error / Exception**                  | Prove where the routine failed, with enough context to reconstruct execution. | **At the point** where the error is detected or handled.                                                        | `invalid mode`, `invalid piece count`            |
