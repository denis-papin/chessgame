---
name: implement-code
description: Implement or modify the code for a feature (and make its tests pass) from the `_ai/` specs. Use when the user wants to build, implement, change, extend, or "make Fxxxx pass" — e.g. "/implement-code F0001", "implement feature F0001", "modify F0002 to match the updated spec", "write the code for F0003 and get the tests green". Implements new code or modifies existing code so the feature and all its specified tests are satisfied, runs the full test suite as the source of truth, and refuses to touch the test definitions or the business spec. Stops and reports in chat on any contradiction, ambiguity, missing requirement, or blocking issue.
---

# Implement or modify a feature from its `_ai` specification

This skill brings the code for a single feature, identified by its `Fxxxx` id
(e.g. `F0001`), into line with its specification — **writing new code where none
exists and modifying existing code where it does** — so that **all the tests
specified for that feature pass**, while treating the spec and the tests as a
fixed contract.

It works the same whether the feature is greenfield (no implementation yet) or
already partly built (the spec or tests changed and the code must catch up):
read the contract, see what the tests demand, then add or change production code
until they pass.

The user invokes it as `/implement-code Fxxxx`. The argument is the feature id.
If no `Fxxxx` is supplied, ask the user which feature to implement before doing
anything else. If more than one `Fxxxx` is given, implement them one at a time in
the order listed.

---

## Operating contract (read this first)

These rules are absolute. They override any urge to "just make it green".

- **The tests are the source of truth.** A correct implementation is one where
  the feature's tests pass *as written*. Validate the implementation by running
  the tests, not by reasoning that the code "looks right".
- **Never modify the test definitions.** Do not edit, delete, weaken, skip, or
  `#[ignore]` / `it.skip` any test, test file, or test spec
  (`IT-Fxxxx.md`, `unit_test_Fxxxx.md`, `integration_test_*.rs`, files under
  `chessgame/src/tests/`, `api-tests/`). Do not change assertions, fixtures, or
  expected values to fit the code.
- **Never modify the business specification.** The `_ai/features/Fxxxx-*/Fxxxx.md`
  feature design and the `_ai/global/*` docs are read-only inputs.
- **Stop and report on any blocker.** If you hit a contradiction, an ambiguity,
  a missing requirement, an impossible-to-satisfy test, or anything that
  prevents a *correct* implementation, **stop** and report it in the chat (see
  [Reporting a blocker](#reporting-a-blocker)). Do not paper over it by editing
  a test or guessing.
- **Implement only the named feature.** Stay within `Fxxxx`'s scope. Touch other
  code only where the feature genuinely requires it, and say so.

The only files this skill writes are **production / source files** (and, where
the spec explicitly calls for new tests that do not yet exist, the test files it
tells you to create — never an existing test).

---

## Where things live

The `_ai/` folder is the single source of truth for *what* to build; the test
specs and test files define *how it is verified*.

| Path | Role |
| --- | --- |
| `_ai/features/Fxxxx-<slug>/Fxxxx.md` | Feature design: goal, gap, inputs, output, flow & routines, **rules** (`B-*` back, `F-*` front), errors, examples, out-of-scope. Opens with a `yaml` block (`id`, `status`, `related_tests`). |
| `_ai/features/Fxxxx-<slug>/IT-Fxxxx.md` | Integration-test spec (black-box, over the REST API). |
| `_ai/features/Fxxxx-<slug>/unit_test_Fxxxx.md` | Unit-test spec (internal routines). |
| `_ai/global/architecture.md` | System split (front `chessgame` :5173, back `fisher-server` :7200, Stockfish), ports, runtime, start commands. |
| `_ai/global/coding-rules.md` | Folder conventions (`infra`/`domain`/`events`; back-end `game/`/`moves/`/`engine/`), logging rules, status-code contracts. |
| `chessgame/src/{infra,domain,events,tests}/` | Front-end source and unit tests (TypeScript + Vite, tested with **vitest**). |
| `fisher-server/src/{main.rs,game/,...}` | Back-end source (Rust / Axum). |
| `api-tests/` | Rust integration tests, part of the same workspace (`cargo test`). |

---

## Workflow

1. **Resolve the feature.** From the `Fxxxx` argument, find
   `_ai/features/Fxxxx-*/`. If the folder does not exist, stop and tell the user
   the feature has no spec.

2. **Read the whole contract before writing code.**
   - Read `Fxxxx.md` end to end — goal, inputs/output, every `B-*`/`F-*` rule,
     the error table, and the worked examples.
   - Read `IT-Fxxxx.md` and `unit_test_Fxxxx.md`.
   - Read the matching test *files* (under `chessgame/src/tests/`, `api-tests/`,
     or wherever `related_tests` points). These are the precise assertions you
     must satisfy — let them drive the design.
   - Skim `_ai/global/architecture.md` and `coding-rules.md` for the folder
     layout, naming, logging, and status-code conventions you must follow.

3. **Check for blockers up front.** Before implementing, confirm the spec and
   the tests are mutually consistent and complete enough to implement correctly.
   If not, **stop and report** (see below) rather than starting.

4. **Plan the implementation.** Map each rule and each test to the code that
   will satisfy it. Respect the architecture: front-end logic split across
   `infra` (REST calls only), `domain` (pure local computing), `events` (page
   events); back-end handlers thin, delegating to business-named subfolders,
   with mandatory logging and the specified HTTP status codes. Server stays
   authoritative for chess rules.

5. **Implement or modify.** Where the feature has no implementation yet, write the
   production code feature-first. Where code already exists, modify it in place to
   match the current spec and tests — extend, refactor, or correct it rather than
   duplicating. Keep changes scoped to `Fxxxx`. Follow the repo conventions
   (strict TypeScript — do not silence `noUnusedLocals` etc.; `Result` over panics
   in Rust request paths; business-oriented names; log service start/end, success,
   failure, and significant branches with the game `uuid`).

6. **Run the complete test suite** (see [Running the tests](#running-the-tests))
   and read the output as the source of truth.

7. **Iterate on the production code only** until the feature's tests pass. When a
   test fails, fix the *implementation* to match the test — never the reverse. If
   a failure reveals that the test and the spec genuinely contradict each other,
   **stop and report**.

8. **Report the result.** Summarise what you implemented, which tests now pass,
   and any follow-ups. Cite the feature (`Feature: Fxxxx — <slug>`).

---

## Running the tests

Run the full suite for the side(s) the feature touches, then confirm nothing
else regressed.

- **Front end (`chessgame`, vitest):**
  `cd chessgame; npm test` (or `npx vitest run`). Also keep the strict build
  clean: `npm run build` / `tsc` must not introduce errors.
- **Back end (`fisher-server` + `api-tests`, Cargo workspace):**
  `cargo test` from the workspace root runs unit and integration tests.
- Integration tests drive `fisher-server` over its REST API and may need the
  server (and, per the spec, the Stockfish engine) available. If a test requires
  a running dependency that is not available in this environment, that is a
  **blocker to report**, not a reason to skip or edit the test.

Use the actual command output — pasted or summarised — as evidence. "Tests pass"
must mean you ran them and saw them pass.

---

## Reporting a blocker

When you hit a contradiction, ambiguity, missing requirement, or anything that
blocks a correct implementation, **stop implementing** and post a clear,
business-readable report in the chat with:

1. **What is blocked** — the rule, test, or behaviour you cannot satisfy.
2. **The conflict** — quote/cite the spec (`Fxxxx.md` rule id) and the test
   (file + assertion) that disagree, or name the requirement that is missing.
3. **Why it blocks a correct implementation** — what you'd have to guess or break
   to proceed.
4. **The available options** — concrete choices for the user (e.g. "interpret
   rule B-3 as X and accept test T2 as-is", "the spec omits the error code for an
   unknown uuid — confirm 404"), with a recommendation if you have one.

Do **not** resolve a blocker by editing a test or the spec. Leave the contract
untouched and let the user decide.

---

## Constraints (summary)

- **Implement or modify the feature; make its tests pass by writing or changing production code.**
- **Read-only on all specs and tests** — `_ai/**`, every test file and test spec.
- **Validate by running the full test suite**, not by inspection.
- **Stop and report** any contradiction, ambiguity, missing requirement, or
  blocker instead of guessing or weakening a test.
- **Follow the repo conventions** (architecture split, naming, logging, status
  codes, strict TypeScript).
- **Always name the `Fxxxx` feature** in the final report.
