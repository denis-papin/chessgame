# Coding rules

Conventions every contributor (human or AI) follows in this repository. They
keep the two modules — the `chessgame` front end and the `fisher-server` back
end — consistent and easy to test. See [architecture.md](architecture.md) for
how the pieces fit together.

---

## 1. Front end — `chessgame` (TypeScript)

A Vite + TypeScript app. The source lives under `chessgame/src/` and is split by
**role**, so that calling APIs, computing locally, and handling page events never
get mixed in the same file.

| Folder | Responsibility | Must **not** |
| --- | --- | --- |
| `infra` | TS routines that call the REST APIs of `fisher-server` | hold game logic |
| `domain` | TS routines for local computing (board model, coordinate maths, move helpers) | touch the DOM or the network |
| `events` | All code reacting to events on the page (clicks, drag/drop, keyboard) | embed business rules — delegate to `domain`/`infra` |
| `tests` | Unit tests for the front module | reach the real back end (mock `infra`) |

Guidelines:

* **One direction of dependency:** `events` → (`domain`, `infra`) → REST.
  `domain` stays pure and framework-free so it is trivial to unit test.
* **Network only in `infra`.** A single place builds requests and parses
  responses, so the API contract is easy to change.
* **TypeScript is strict.** `tsconfig.json` enables `noUnusedLocals`,
  `noUnusedParameters` and `noFallthroughCasesInSwitch` — keep the build clean,
  do not silence them.
* **No secrets or hard-coded hosts in source.** The back-end base URL belongs in
  one config/constant, not scattered across `infra` calls.

---

## 2. Back end — `fisher-server` (Rust / Axum)

The back end is the `fisher-server` module, written in **Rust / Axum** to provide
RESTful APIs.

Structure:

* **Entry points live in `main`**, but each one calls a **delegate routine** in a
  subfolder whose name is **business-oriented** (e.g. `game/`, `moves/`,
  `engine/`) — not technical.
* The code follows current Rust best practice, **except** that line length is not
  tightly limited — favour readable, descriptive lines over wrapping.
* Keep handlers thin: parse/validate the request, delegate to the business
  routine, map the result to an HTTP response.

### 2.1 Logging

Logging is **mandatory**. Emit a `log info` (or appropriate level):

* every time a service **starts** and **ends**;
* every time a meaningful action is **completed**;
* every time a meaningful action has **failed**;
* every time a process is **routed onto an important path** (a significant branch).

Guidelines:

* Include the `uuid` of the current game in log lines so a game can be traced
  end to end.
* Use levels deliberately: `info` for the events above, `warn` for recoverable
  problems, `error` for failures that abort an action.
* Log facts, not secrets — never log full request bodies that could contain
  sensitive data.

### 2.2 Errors

* Validate input at the edge of a handler and return a clear HTTP status
  (`400` bad move, `404` unknown game `uuid`, `409` not your turn, `5xx` engine
  failure).
* Prefer `Result` with explicit error types over panics in request paths.

---

## 3. Integration tests — `api-tests`

A Rust module that is **part of the same Rust workspace** as `fisher-server`.

* It **groups the integration tests** for the back-end features.
* An API test (a.k.a. integration test) uses **only public application APIs** —
  it drives `fisher-server` through its REST endpoints, never through internal
  functions or private state.
* One test file per feature, named to match the feature (e.g.
  `integration_test_F0001.rs`), with a spec in
  `_ai/<FEATURE>/integration_test_*.md`.
* Tests should be self-contained: start a game via the API, play through it, and
  assert on the responses.

---

## 4. General conventions

* **Business-oriented names** everywhere — folders, modules, functions describe
  *what* they do for chess, not *how* they are built.
* **The server is authoritative.** Chess rules and move validation live in
  `fisher-server`; the front end stays thin and trusts the returned state.
* **Pin the API contract first** (see [architecture.md](architecture.md) §4–§5)
  so the front, the back, and `api-tests` can be developed in parallel.
* **Small, focused commits** that keep both modules in a buildable state.

---
