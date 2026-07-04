---
name: implement-proof-logs
description: Implement or refactor a feature's logs into proof logs, following the `_ai/global/proof-logs.md` guide. Use when the user wants to add, fix, audit, or refactor logging so it becomes proof of execution — e.g. "/implement-proof-logs F0001", "turn the logs in start_game into proof logs", "audit the proof logs for F0002", "add the missing proof logs to fisher-server". Brings the emitted logs into line with the eight placement rules, the three levels (INFO/WARN/ERROR), the stream tag, and the follower keys, then confirms the build and tests stay green. Read-only on specs and tests.
---

# Implement or refactor logs into proof logs

This skill brings the **logs** of a feature into line with the project's proof-of-execution
model, defined in [proof-logs.md](_ai/global/proof-logs.md). In this project every log is a
**proof log** — one piece of execution evidence kept on in production — and the logs of one
execution context form a **proof line**, the ordered trace of the path the code actually took.
There is no separate "debug logging": a log either proves something worth keeping in production
or it should not exist.

The skill works the same whether the feature has **no logs yet** (add them), **partial or ad-hoc
logs** (refactor them), or **logs that drifted from the guide** (correct them): read the guide and
the feature, decide where proof logs belong, then add or change the emitting code until the logs
match the model — without touching the specs or the tests.

The user invokes it as `/implement-proof-logs <target>`, where `<target>` is a feature id
(`Fxxxx`), a module/path (e.g. `fisher-server/src/game/`), or a description of the flow to cover.
If no target is supplied, ask which feature or module to work on before doing anything else.

---

## Operating contract (read this first)

These rules are absolute.

- **`proof-logs.md` is the authority.** [`_ai/global/proof-logs.md`](_ai/global/proof-logs.md)
  defines what a proof log and a proof line are, what to log, the levels, the stream tag, the
  `follower` keys, and the eight placement rules. Follow it; do not invent a different logging
  convention.
- **Cover every placement rule — no "light" proof lines.** The common failure is emitting only the
  obvious logs (**Feature Entry**, **State Change**, **Feature Exit**) and silently dropping the
  **Business Decision (4)**, **Invariant / Rule Check (6)**, and **Business Milestone (7)** — and
  leaving the exit without its `result`. That is the mistake this skill exists to prevent. **All
  eight** placement rules in [`proof-logs.md`](_ai/global/proof-logs.md) must be *explicitly
  accounted for* on the target flow: either a proof log is emitted at that point, or you record a
  one-line reason it is genuinely **N/A** for this feature, grounded in the spec — exactly as the
  guide's own table marks rule 3 *"none in START-A-GAME"*. A rule may **never** be skipped
  silently. You prove this by building the placement coverage matrix (Workflow step 4) and showing
  it in the report; if any of the eight rows is blank, you are not done.
- **Logs are proof, not debug output.** Only `INFO`, `WARN`, and `ERROR` exist. **Never** emit
  `DEBUG` or `TRACE`, and remove any you find. A log must prove a real execution event.
- **Read-only on specs and tests.** `_ai/**`, every test spec (`IT-Fxxxx.md`,
  `unit_test_Fxxxx.md`) and every test file (`api-tests/`, `chessgame/src/tests/`,
  `integration_test_*.rs`) are read-only inputs. Never edit, weaken, or skip a test to make logs
  fit — if a test asserts on log output and your change would break it, that is a **blocker to
  report**, not a test to change.
- **Do not change business behaviour.** This skill adds and adjusts *logging*; it must not alter
  control flow, results, HTTP status codes, or the data returned. Adding a log at a branch is fine;
  moving the branch is not.
- **Never log secrets.** Log facts, not full request bodies or anything that could carry sensitive
  data (proof-logs.md, *What to log*).
- **Stop and report on any blocker** (a contradiction with the guide, an ambiguous placement, a
  test that pins log text you'd have to break, missing follower infrastructure you can't add in
  scope). See [Reporting a blocker](#reporting-a-blocker).

The only files this skill writes are the **production / source files** that emit logs (today,
almost always under `fisher-server/src/`, since logging is mandatory in `fisher-server`).

---

## Where things live

| Path | Role |
| --- | --- |
| [`_ai/global/proof-logs.md`](_ai/global/proof-logs.md) | **The guide.** Proof log & proof line, what/when to log, levels, stream tag, `follower` keys, the eight placement rules. Read it every time. |
| `_ai/global/coding-rules.md` | §2.1 makes logging mandatory in `fisher-server` and points to the guide. |
| `_ai/features/Fxxxx-<slug>/Fxxxx.md` | Feature design — its flow, business decisions, state changes, milestones, and errors are exactly the points that need proof logs. |
| `_ai/features/Fxxxx-<slug>/IT-Fxxxx.md` | Integration-test spec — running the IT is what produces the feature's **set of proof lines**. |
| `fisher-server/src/proof_log.rs` | **The proof-log toolkit you must use.** Defines the `log_info_f!` / `log_warn_f!` / `log_error_f!` macros and the closed `LogFeature` stream enum. Every proof log goes through these. |
| `fisher-server/src/{main.rs,lib.rs,game/,...}` | Back-end source: the handlers and business delegates where proof logs are emitted (via the macros above). |
| `fisher-server/src/main.rs` | Builds the `tracing` subscriber — the place that controls what the log line shows (target/module, and eventually the `follower` trailer). |

---

## Always emit through the proof-log macros

Do **not** call `tracing::info!` / `warn!` / `error!` directly. Every proof log in
`fisher-server` goes through the macros in
[`proof_log.rs`](fisher-server/src/proof_log.rs), which stamp the log with its **stream** for you:

- **`log_info_f!` / `log_warn_f!` / `log_error_f!`** — one per level. There is no `log_debug_f!`
  or `log_trace_f!` on purpose (only three levels exist).
- **The first three arguments are the `follower` keys**, in order: the **stream**
  (`LogFeature::_.as_str()`), the **session id**, and the **request tracking id**. The macro renders
  them as the `follower=[[session][request][stream]]` trailer. Everything after is a normal
  `tracing` message — format string, format args, and optional extra fields (`uuid = %uuid`, …).

```rust
use crate::proof_log::LogFeature;

// `session` / `tracking` are the follower ids, read once at the handler edge and
// threaded through every log of the request. A full proof line covers each
// applicable placement rule — not just entry/state-change/exit:
log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, "start-game requested");                              // 1 Feature Entry
log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, mode = mode.as_str(), "layout mode selected");        // 4 Business Decision
log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, pieces = pieces, piece_count_valid = true, "piece count validated"); // 6 Invariant / Rule Check
log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, uuid = %uuid, mode = mode.as_str(), "board generated"); // 7 Business Milestone
log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, uuid = %uuid, mode = mode.as_str(), "game created");  // 5 State Change
log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, uuid = %uuid, result = "SUCCESS", "start-game served"); // 2 Feature Exit (with result)
log_warn_f!(LogFeature::StartAGame.as_str(), session, tracking, error = message, "request rejected");                // 8 Error / Exception
// 3 External Boundary — N/A for START-A-GAME (no engine call), as the guide marks it.
```

**Adding a feature = adding a stream.** When you cover a new feature, add its variant to the
`LogFeature` enum in `proof_log.rs` (e.g. `AccoView => "ACCO-VIEW"`) rather than passing a raw
string — that is what keeps the set of streams closed and discoverable (proof-logs.md, "Stream
codes"). Never hand a literal string to the macro.

The macros are `#[macro_export]`ed and the module is `#[macro_use]`, so they are available
unqualified across the crate (and importable from `fisher_server` in `api-tests`). You still need
`use crate::proof_log::LogFeature;` in each module that names a stream.

---

## The proof log model in one screen

From [proof-logs.md](_ai/global/proof-logs.md), the parts you must get right:

- **What to log** — emit a proof log when a service **starts** and **ends**, when a meaningful
  action **completes** or **fails**, and when the flow is **routed onto a significant branch**.
  Carry the game **`uuid`** in every log so a game traces end to end.
- **Levels (only three).** `INFO` = the proof events (entry/exit, decisions, milestones, state
  changes). `WARN` = a recoverable problem the flow handled and continued past. `ERROR` = a failure
  that aborts the action. No `DEBUG`/`TRACE`.
- **Stream tag.** Every log carries the **feature name** as its stream (e.g. `START-A-GAME`) — a
  `grep`-able label for one business flow, and one of the three keys that reconstruct a proof line.
  The macros stamp it for you from the closed `LogFeature` enum (see
  [above](#always-emit-through-the-proof-log-macros)); keep that enum the single source of streams
  so a code can't be misspelled into existence.
- **The eight placement rules** — logs must sit at the same predictable points in every feature so
  a production proof line is comparable against the proven one:

  | # | Placement rule | Where | Find it in `Fxxxx.md` |
  | - | -------------- | ----- | --------------------- |
  | 1 | **Feature Entry** | before the routine starts | the handler/routine in [Flow & routines] |
  | 2 | **Feature Exit** | after it completes (success *or* failure) — **carry a `result` field** (`result="SUCCESS"` / failure) | end of that routine |
  | 3 | **External Boundary** | before the request to / after the response from an external system | any external call in [Flow] (often **N/A** — no engine call) |
  | 4 | **Business Decision** | immediately after the decision is made (log which path and why) | [Inputs] / branch choices in [Flow] (e.g. `mode`) |
  | 5 | **State Change** | immediately after a persistent state actually changed | registry/persistence writes in [Flow] |
  | 6 | **Invariant / Rule Check** | immediately after an important rule is evaluated — **log the result even when it passes**, not only on failure | the `B-*` / `F-*` entries in [Rules] (e.g. piece-count valid) |
  | 7 | **Business Milestone** | after a major logical step completes | the major steps in the [Flow & routines] walkthrough (e.g. board generated) |
  | 8 | **Error / Exception** | at the point the error is detected or handled, with enough context | every row of the [Errors] table |

  The guide's placement table lists a concrete **START-A-GAME example for each rule** — use it as
  the reference shape for the log you emit at each point.

- **The `follower` trailer** — the triple `[[session][request][stream]]` (session id, request
  tracking id, stream) plus the `uuid` are the keys that reconstruct one proof line. The `log_*_f!`
  macros render the whole trailer for you from the three keys you pass. Source the **session** and
  **request** ids once at the handler edge (today from the `x-session-id` / `x-tracking-id` request
  headers, falling back to `-` when a caller has not supplied them) and thread them through every
  log of that request; end-to-end propagation of these ids across services is still being wired up,
  so a `-` in the trailer is expected until a caller sends them.

---

## Workflow

1. **Read the guide, every time.** Read [`_ai/global/proof-logs.md`](_ai/global/proof-logs.md)
   in full — it is the contract for this skill and it evolves.

2. **Resolve the target.** From the argument, find the feature (`_ai/features/Fxxxx-*/`) and/or the
   source it maps to. Read the feature's `Fxxxx.md` to learn its flow — its entry/exit, its
   business decisions, its state changes, its milestones, and its error table are the proof-log
   points. Note the feature name; that is the **stream**.

3. **Audit the current logs.** Find every existing log statement in the target
   (`Grep` for `tracing::`, `info!`, `warn!`, `error!`, `debug!`, `trace!`, `println!`, `log`, and
   the `log_*_f!` macros). For each one classify it against the model: emitted through a `log_*_f!`
   macro (not raw `tracing::`)? right level? carries the `uuid`? carries the stream? sits at one of
   the eight placement points? Note every gap (a raw `tracing::` call that should be a macro, a
   missing entry/exit, a decision that isn't logged, a `DEBUG` that must go, a level that's wrong, a
   missing `uuid`).

4. **Build the placement coverage matrix (mandatory — this is the step that prevents a light proof
   line).** Before writing any log, walk **all eight placement rules** from
   [`proof-logs.md`](_ai/global/proof-logs.md) against the target flow, reading the feature's
   `Fxxxx.md` for the point that triggers each one (see the *Find it in `Fxxxx.md`* column above):
   its **entry/exit** ([Flow & routines]), each **business decision** ([Inputs] / branch choices),
   each **state change**, each **milestone** ([Flow & routines] walkthrough), each **invariant /
   rule check** ([Rules]), and every row of the **[Errors]** table. Produce a table with **one row
   per rule (all eight present)**:

   | # | Placement rule | Spec point (from `Fxxxx.md`) | Proof log to emit (message + fields) | Code site (file · fn) |
   | - | -------------- | ---------------------------- | ------------------------------------ | --------------------- |

   For each rule either fill in the log, or write **`N/A — <reason from the spec>`** (e.g. rule 3:
   *"N/A — this feature makes no external/engine call"*, mirroring the guide's *"none in
   START-A-GAME"*). This matrix **is** the proof line you are implementing; confirm it lines up with
   the IT scenarios (each IT scenario should yield one clean proof line). **If any of the eight rows
   is blank — neither a log nor a justified `N/A` — you are not done.** Rules 4, 6, and 7 (Business
   Decision, Invariant/Rule Check, Business Milestone) and the exit's `result` field are the ones
   routinely forgotten — check them explicitly.

5. **Implement or refactor the logs — one per matrix row.** Emit every proof log through the
   `log_info_f!` / `log_warn_f!` / `log_error_f!` macros with a `LogFeature` stream — convert any raw
   `tracing::` calls over to them. Add a log for **every non-`N/A` row of the matrix** (not just the
   obvious three), correct wrong levels, remove `DEBUG`/`TRACE` and any non-proof noise, attach the
   **`uuid`** to every log, and give the Feature Exit its `result`. If the feature has no
   `LogFeature` variant yet, add one to [`proof_log.rs`](fisher-server/src/proof_log.rs) rather than
   passing a raw string. Keep the emitting code business-readable and change **only logging** — never
   control flow or results.

6. **Verify build and tests stay green.** Logging changes must not break anything (see
   [Verifying](#verifying)). Read the actual output as evidence.

7. **Report — present the coverage matrix.** Show the final placement coverage matrix with **all
   eight rules** and, for each, the proof log now emitted **or** the justified `N/A`. State
   explicitly that no rule was silently skipped and that the Feature Exit carries its `result`.
   Note any `DEBUG`/`TRACE` removed and levels corrected, paste the actual emitted proof line(s) as
   evidence, and flag anything left pending (e.g. the `follower` session/request plumbing, if out of
   scope). Cite the feature/stream (`Feature: Fxxxx — <slug>`, `stream: START-A-GAME`).

---

## Verifying

Logging is cross-cutting, so the bar is "nothing regressed and the proof logs actually appear".

- **Build & tests stay green.** Back end: `cargo test` from the workspace root (unit +
  `api-tests`); `cargo build` must stay clean (no new warnings from your log statements). Front end,
  if touched: `cd chessgame; npm test` and a clean `npm run build`.
- **The proof logs appear, in order, covering every rule.** Where practical, exercise the flow (an
  integration test, or the running server per `architecture.md`) and read the emitted logs: confirm
  the feature entry, **the business decision(s)**, **the invariant/rule check(s)**, the state
  change(s), **the milestone(s)**, and the exit (with its `result`) are all present, at the right
  level, each carrying the `uuid` and the stream — i.e. a readable proof line. Check the live proof
  line against the coverage matrix: every non-`N/A` row must show up. A proof line missing rules 4,
  6, or 7 is the light-proof-line regression this skill must catch.
- If a test asserts on log content and your change conflicts with it, **stop and report** — do not
  edit the test.

Use the real command output as evidence. "The proof logs are in place" must mean you ran it and saw them.

---

## Reporting a blocker

When you hit something that blocks correct proof logs, **stop** and post a clear, business-readable
report with:

1. **What is blocked** — the placement, level, or key you cannot get right.
2. **The conflict** — cite the guide (`proof-logs.md`, section/rule) and the code or test that
   disagrees.
3. **Why it blocks correct proof** — what you'd have to guess, break, or over-build to proceed
   (e.g. the `follower` session/request ids need front-to-back plumbing that isn't in this target).
4. **The options** — concrete choices for the user, with a recommendation (e.g. "add the stream tag
   and levels now; track the session/request `follower` fields as a separate infra task").

Do **not** resolve a blocker by editing a test or the spec, or by inventing a logging convention
the guide doesn't define.

---

## Constraints (summary)

- **Make the logs proof logs** per [`_ai/global/proof-logs.md`](_ai/global/proof-logs.md) — the
  eight placement rules, the three levels, the stream tag, the `uuid`, and the `follower` keys.
- **Account for all eight placement rules** via the coverage matrix (Workflow step 4): each rule
  gets a proof log or a spec-grounded `N/A` — never a silent gap. Rules 4/6/7 and the exit `result`
  are the ones to check explicitly. Present the matrix in the report.
- **Emit through the macros.** Use `log_info_f!` / `log_warn_f!` / `log_error_f!` from
  [`proof_log.rs`](fisher-server/src/proof_log.rs) with a `LogFeature` stream — never a raw
  `tracing::` call and never a literal stream string. New feature ⇒ new `LogFeature` variant.
- **Only `INFO`/`WARN`/`ERROR`.** Remove every `DEBUG`/`TRACE`; a log proves something or it goes.
- **Change logging only** — never business behaviour, results, or status codes.
- **Read-only on all specs and tests** (`_ai/**`, every test spec and test file). A test that pins
  log output is a blocker to report, not a test to change.
- **Never log secrets.**
- **Verify** the build and tests stay green and the proof line actually appears.
- **Stop and report** any contradiction, ambiguity, or missing infrastructure instead of guessing.
- **Always name the feature and its stream** in the final report.
