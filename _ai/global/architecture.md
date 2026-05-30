# Architecture

The **ChessGame** project lets a human play chess in the browser against a
Stockfish engine. It is split into three cooperating processes: a TypeScript
front end, a Rust back end, and the Stockfish engine.

> Naming note: the back end is called **fisher-server** — a pun on *Fischer*
> (the chess champion) and *fish* (Stock**fish**). It is the brain that talks to
> the engine on the front end's behalf.

---

## 1. Overview

```
        ┌──────────────────┐   REST / JSON    ┌──────────────────┐   UCI / stdio   ┌──────────────────┐
        │    chessgame     │ ───────────────► │   fisher-server  │ ──────────────► │    Stockfish     │
        │   (front, TS)    │ ◄─────────────── │   (back, Rust)   │ ◄────────────── │     (engine)     │
        │  Vite · :5173    │   board state    │  Axum · :7200    │   best move     │   external proc  │
        └──────────────────┘                  └──────────────────┘                 └──────────────────┘
              browser                            game authority                       move calculation
```

| Layer | Module | Tech | Port | Responsibility |
| --- | --- | --- | --- | --- |
| Front | `chessgame` | TypeScript + Vite | `5173` | Render the board, capture user moves, call the back end |
| Back | `fisher-server` | Rust + Axum | `7200` | Hold game state, validate moves, broker the engine |
| Engine | Stockfish | UCI binary | _TBD_ | Compute the best reply for a given position |

---

## 2. Components

### 2.1 `chessgame` — front end

A Vite-powered TypeScript single-page app served at **http://localhost:5173/**.

Responsibilities:

* Render the board and pieces, and reflect the global position served by the back end.
* Let the player interact: select a piece, drag/drop or click-to-move.
* Refresh the board on demand from the global position held by `fisher-server`.
* Show side panels: move list, captured pieces, whose turn it is.
* Talk to the `fisher-server` REST APIs (no direct contact with Stockfish).

Code layout (per [coding-rules.md](coding-rules.md)):

| Folder | Purpose |
| --- | --- |
| `infra` | Routines that call the REST APIs |
| `domain` | Local computation (board model, coordinate maths, move helpers) |
| `events` | Page event handling (clicks, drag/drop, keyboard) |
| `tests` | Unit tests for the front module |

### 2.2 `fisher-server` — back end

A Rust / Axum service exposing RESTful APIs at **http://localhost:7200/**.
It is the **single source of truth** for a game in progress.

Responsibilities:

* Keep the ordered list of moves played.
* Keep the global position of every piece on the board.
* Know the `uuid` of the current game.
* Keep the list of captured pieces.
* Communicate with the Stockfish engine to obtain the best next move.

Code layout (per [coding-rules.md](coding-rules.md)):

* Entry points live in `main`, but each delegates to a routine in a
  business-oriented subfolder (e.g. `game/`, `engine/`, `moves/`).
* Logging is mandatory: when a service starts/ends, when a meaningful action
  succeeds or fails, and when a process takes an important branch.

### 2.3 Stockfish — engine

An external process that speaks the **UCI** protocol over stdio. The
`fisher-server` feeds it a position (FEN or move list), asks it to think, and
reads back the best move. The front end never talks to it directly.

To start the stockfish engine :
docker run \
  --name stockfish \
  -p 3000:3000 \
  ghcr.io/samuraitruong/stockfish-docker:14.1 \
  stockfish

---

## 3. Request flow — a single move

```
Player drags pawn e2 → e4
        │
        ▼
[chessgame] events/ captures the move
        │  POST /games/{uuid}/moves  { from: "e2", to: "e4" }
        ▼
[fisher-server] validates the move, updates position + move list + captures
        │  asks the engine for the reply
        ▼
[Stockfish]  position ... / go  → bestmove e7e5
        │
        ▼
[fisher-server] applies the engine move, returns the new global position
        │  200 OK { position, moves, captured, turn }
        ▼
[chessgame] refreshes the board
```

---

## 4. Data model (proposed)

The shapes the two sides agree on. Suggested as a starting contract.

```jsonc
// Game state returned by fisher-server
{
  "uuid": "5f1c…",            // current game id
  "turn": "white",           // side to move
  "position": "rnbqkbnr/…",  // FEN string, or an 8×8 piece grid
  "moves": [                 // ordered history
    { "ply": 1, "from": "e2", "to": "e4", "san": "e4" }
  ],
  "captured": {              // pieces removed from the board
    "white": ["p", "n"],
    "black": ["P"]
  },
  "status": "in_progress"    // in_progress | checkmate | stalemate | draw
}
```

---

## 5. REST API surface (proposed)

A first cut of the endpoints `fisher-server` would expose. Aligns with
feature **F0001 — start a game**.

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/games` | Start a new game, returns a fresh `uuid` and the start position |
| `GET` | `/games/{uuid}` | Fetch the full current game state |
| `GET` | `/games/{uuid}/position` | Fetch just the global position (cheap board refresh) |
| `POST` | `/games/{uuid}/moves` | Play a move; server replies and returns the new state |
| `GET` | `/games/{uuid}/moves` | List the move history |
| `GET` | `/health` | Liveness probe (also reports engine availability) |

---

## 6. Runtime & development

| Service | URL | Start command |
| --- | --- | --- |
| Front | http://localhost:5173/ | `cd chessgame && npm run dev` |
| Back | http://localhost:7200/ | `cd fisher-server && cargo run` |
| Engine | _TBD_ | launched/managed by `fisher-server` |

The front talks to the back across origins, so `fisher-server` must enable
**CORS** for `http://localhost:5173` during development.

---

## 7. Open questions & ideas

Things worth deciding early:

* **Engine transport.** Spawn Stockfish as a child process and speak UCI over
  stdio (simplest), or run it as a separate networked service? This drives the
  "undef port yet" note.
* **Move validation.** Where do chess rules live — in `fisher-server` (so the
  server is authoritative and the front stays thin), or shared? Recommended:
  validate authoritatively on the server.
* **State storage.** In-memory map of `uuid → game` is enough for a demo;
  add persistence (file/SQLite) only if games must survive a restart.
* **Engine difficulty.** Expose Stockfish skill level / think time as a game
  setting (`POST /games` parameter).
* **Real-time updates.** REST polling works for a turn-based game; consider
  Server-Sent Events or WebSocket later if you want spectators or live sync.
* **API contract first.** Pin the JSON shapes in §4–§5 so the front and the
  `api-tests` integration suite can be built in parallel.

---

## 8. Related documents

* [coding-rules.md](coding-rules.md) — folder conventions and logging rules.
* [F0001 — start a game](../features/F0001-start-a-game/F0001.md) — first feature spec.
* [IT-F0001](../features/F0001-start-a-game/IT-F0001.md) — its integration tests.
