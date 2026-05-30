```yaml
id: UT-F0001
title: Front unit tests — render the board from a start-game Overview
type: unit-test
status: draft
target_stream: GAME-LIFECYCLE
related_feature: F0001
naming_prefix: UT-F0001
language: typescript
```

# Coverage goal

Local, fast validation of the **front-end** logic that turns a `GET /start-game` response into a rendered board, exercised at the **module boundary** with no real back end and no real browser. The tests run under Vitest (jsdom) and call the front's own `domain`, render, and `infra` functions directly. The board model under test is the project's own `Overview` JSON (an 8×8 array of piece strings plus per-colour castling), **not** FEN.

What is asserted:

- the `domain` cell iterator maps the standard `Overview` to the correct 32 occupied squares — proves **F-3**/**F-5**;
- it maps a sparse `Overview` (empty cells, scattered pieces) correctly — proves **F-3**;
- the square-colour function makes `a1` dark and follows `(file+rank)` parity — proves **F-2**;
- the board model is 64 squares with correct coordinates, white at the bottom — proves **F-1**;
- rendering builds 64 square elements first, then places piece elements onto them that match the `Overview` exactly — proves **F-4**/**F-5**;
- a re-render (rematch / new `pieces` count) reuses the same 64-square grid and swaps only the pieces — proves **F-4** (grid reused, never rebuilt or duplicated);
- file marks `a`–`h` and rank marks `1`–`8` are present on the edges — proves **F-6**;
- `infra.startGame()` builds the right query (mode + `pieces`), parses `{ uuid, mode, overview }` from a mocked HTTP response, and touches the network only through that one seam — proves **F-8**.

The rules referenced above are defined in [F0001.md](F0001.md#rules).

# System under test

The named front routines from [F0001.md → Flow & routines](F0001.md#flow--routines), called directly (this is a unit-test spec — the boundary is the function, not the public REST API):

- `domain/overview.ts` — the `Overview` type (`{ board: string[][]; white: Castle; black: Castle }`), `cells(ov: Overview): Array<{ square: string; piece: string }>` (occupied squares only, mapping `board[r][c]` → file `a`+c, rank `8`−r), `squareColor(square: string): "light" | "dark"`, and `buildBoard(ov: Overview): BoardModel` (64 cells with coordinate + colour + optional piece).
- `events/renderBoard.ts` — `renderBoard(root: HTMLElement, model: BoardModel): void`, which writes the 8×8 grid and edge labels into a jsdom `HTMLElement`.
- `infra/startGame.ts` — `startGame(opts?: { mode?: "standard" | "random"; pieces?: number }): Promise<{ uuid: string; mode: string; overview: Overview }>`, with `globalThis.fetch` replaced by a Vitest mock (`vi.fn`). It encodes the mode and (for `random`) the single `pieces` count into the query string. **The back end is never contacted**; per [coding-rules.md](../../global/coding-rules.md) §1 front tests mock `infra`'s network.
- `events/onPageLoad.ts` — `onPageLoad(): Promise<void>`, the orchestrator wired to `DOMContentLoaded`. It reads the page's mode + piece-count controls and forwards them to `startGame`. Tested with `startGame` and `renderBoard` replaced by spies so the test sees the call arguments and order, not the network or real DOM writes.

Boundary: `domain` is pure (no DOM, no network) and is tested by calling it and inspecting its return value. `renderBoard` is tested against a jsdom `document` — assertions read the produced DOM (`querySelectorAll`, `dataset`), never internal variables. `startGame` and `onPageLoad` are tested with a stubbed `fetch`/spies; no socket is opened.

Three `Overview` fixtures are reused across cases:

```ts
const STANDARD: Overview = {
  board: [
    ["r","n","b","q","k","b","n","r"],
    ["p","p","p","p","p","p","p","p"],
    ["","","","","","","",""],
    ["","","","","","","",""],
    ["","","","","","","",""],
    ["","","","","","","",""],
    ["P","P","P","P","P","P","P","P"],
    ["R","N","B","Q","K","B","N","R"],
  ],
  white: "both", black: "both",
};

const SPARSE: Overview = {
  board: [
    ["","","","","k","","",""],   // k e8
    ["","","","","","","",""],
    ["","","","","","","",""],
    ["","","N","","","","",""],   // N c5
    ["","","","","","q","",""],   // q f4
    ["","","","","","","",""],
    ["","","","","P","","",""],   // P e2
    ["","","","","K","","",""],   // K e1
  ],
  white: "none", black: "none",
};

// A random layout with pieces=4 (4 white + 4 black, one king each) —
// matches the `?mode=random&pieces=4` example in F0001.
const RANDOM4: Overview = {
  board: [
    ["","","","","k","","",""],   // k e8
    ["","","","p","","","",""],   // p d7
    ["","n","","","","","",""],   // n b6
    ["","","N","","","","",""],   // N c5
    ["","","","","","q","",""],   // q f4
    ["","","","","","","",""],
    ["","","","","P","","",""],   // P e2
    ["","","","Q","K","","",""],  // Q d1, K e1
  ],
  white: "none", black: "none",
};
```

# Test cases

## TC-UT-F0001-001 — iterate the standard Overview into 32 squares

- **Given:** the `STANDARD` fixture.
- **Input:** `cells(STANDARD)`.
- **When:** the function walks `board` rows `0`→`7` (rank 8→1), columns `0`→`7` (file a→h), skipping `""` cells.
- **Then:**
  1. it returns exactly 32 occupied entries — proves **F-5** (no extra/missing pieces);
  2. `a8` holds `"r"`, `e8` holds `"k"`, `e1` holds `"K"`, `a1` holds `"R"`, `h2` holds `"P"` — proves **F-3** (row/col → square mapping and case = colour);
  3. ranks 3–6 contribute no entries — proves **F-3** (empty rows yield no pieces).

## TC-UT-F0001-002 — iterate a sparse Overview with empty cells

- **Given:** the `SPARSE` fixture.

  | Square | Expected piece |
  |--------|----------------|
  | `e8`   | `"k"`          |
  | `c5`   | `"N"`          |
  | `f4`   | `"q"`          |
  | `e2`   | `"P"`          |
  | `e1`   | `"K"`          |

- **Input:** `cells(SPARSE)`.
- **When:** the function skips `""` cells and emits the occupied ones.
- **Then:**
  1. it returns exactly 5 entries, one per table row at the listed square — proves **F-3** (`board[r][c]` maps to the correct square);
  2. no other square is occupied — proves **F-5**.

## TC-UT-F0001-003 — square colour: a1 is dark and parity holds

- **Given:** no fixture; pure coordinate input.

  | Square | `(file+rank)` | Expected |
  |--------|---------------|----------|
  | `a1`   | 2 (even)      | `dark`   |
  | `h1`   | 9 (odd)       | `light`  |
  | `a8`   | 9 (odd)       | `light`  |
  | `h8`   | 16 (even)     | `dark`   |
  | `e4`   | 9 (odd)       | `light`  |

- **Input:** `squareColor(square)` for each row.
- **When:** the function computes `(fileIndex + rankIndex) % 2` with `a`=1…`h`=8.
- **Then:**
  1. each call returns the Expected value — proves **F-2** (`a1` dark, even sum = dark).

## TC-UT-F0001-004 — board model is 64 cells, white at the bottom

- **Given:** the `STANDARD` fixture.
- **Input:** `buildBoard(STANDARD)`.
- **When:** the model is assembled.
- **Then:**
  1. it contains exactly 64 cells with unique coordinates `a1`…`h8` — proves **F-1**;
  2. each cell's colour equals `squareColor(coord)` — proves **F-2** consistency in the model;
  3. iterating the model top-to-bottom yields rank `8` first and rank `1` last, with `a1` at the bottom-left — proves **F-1** (white at the bottom).

## TC-UT-F0001-005 — first render builds the 64 squares, then places pieces that match the Overview

- **Given:** a jsdom `document` whose **mount point** is an empty `<div id="board">` (the host element — it holds no squares yet), and the **populated** model `buildBoard(STANDARD)`, which already carries all 32 pieces. The empty element is the render target; the model is not empty.
- **Input:** `renderBoard(root, board)` where `root` is the `#board` mount point.
- **When:** the renderer lays the 64 square elements into the empty `root` first, then attaches each piece to its existing square — so when piece placement starts the grid is already built, never empty. (Re-rendering into an already-built grid is covered by [TC-009](#tc-ut-f0001-009--re-render-rematch--new-pieces-count-reuses-the-grid-and-swaps-the-pieces).)
- **Then:**
  1. `root.querySelectorAll('[data-square]')` has length 64 — proves **F-1**;
  2. every `[data-piece]` element is a child of a `[data-square]` element (none is attached directly to `root`) — proves **F-4** (pieces render onto the already-built grid, not into an empty container);
  3. the count of piece elements (`[data-piece]`) equals 32 and each one's `dataset.piece` and parent square match `cells(STANDARD)` — proves **F-5** (rendered pieces match the `Overview` exactly);
  4. the square `[data-square="a1"]` carries the dark class/marker and `[data-square="h1"]` the light one — proves **F-2** in the DOM.

## TC-UT-F0001-006 — edge labels a–h and 1–8 are rendered

- **Given:** a jsdom `document` and a rendered standard board (as in TC-005).
- **Input:** read the rendered DOM.
- **When:** the test collects the file-label and rank-label text content.
- **Then:**
  1. the file labels along the bottom edge read `a b c d e f g h` in order — proves **F-6**;
  2. the rank labels along the left edge read `8 7 6 5 4 3 2 1` (top to bottom) — proves **F-6**.

## TC-UT-F0001-007 — infra.startGame parses the response and uses only the network seam

- **Given:** `globalThis.fetch` replaced by `vi.fn().mockResolvedValue(Response with { uuid, mode, overview })` returning `{ uuid: "…", mode: "standard", overview: STANDARD }`.

  | Call argument                            | Expected request URL contains |
  |------------------------------------------|-------------------------------|
  | `startGame()`                            | `/start-game` (no query) |
  | `startGame({ mode: "random", pieces: 4 })` | `/start-game?mode=random&pieces=4` |

- **Input:** `await startGame()` and `await startGame({ mode: "random", pieces: 4 })`.
- **When:** `infra` builds the request and parses the JSON body.
- **Then:**
  1. the resolved value deep-equals `{ uuid, mode, overview }` from the mock — proves `infra` parses the F0001 [Output](F0001.md#output) shape (including the nested `Overview`);
  2. `fetch` was called once per `startGame` call, with a URL matching the table — proves **F-8** (the single network seam; no other module calls `fetch`);
  3. no DOM node is created by `infra` — proves the role split (`infra` ≠ render) from [coding-rules.md](../../global/coding-rules.md) §1.

## TC-UT-F0001-008 — onPageLoad orchestrates startGame → buildBoard → renderBoard

- **Given:** a jsdom `document` with a `<div id="board">` mount point, `infra.startGame` spied/stubbed to resolve `{ uuid: "…", mode: "standard", overview: STANDARD }`, and `events.renderBoard` spied. This is the **key routine** — the one that wires DOM load, the local model build, and the API call together (see the [sequence diagram](F0001.md#flow--routines)).
- **Input:** `await onPageLoad()`.
- **When:** the orchestrator runs the flow.
- **Then:**
  1. `startGame` is called exactly once, with the mode and `pieces` count read from the page controls — proves the API seam is driven from the entry point with the page's chosen values (the count is caller input, rule **B-3**);
  2. `renderBoard` is called once, after `startGame` resolves, with the `#board` root and a `BoardModel` whose occupied cells deep-equal `cells(STANDARD)` — proves `onPageLoad` feeds the parsed `Overview` through `buildBoard` into the renderer (the `startGame → buildBoard → renderBoard` order of **F-8** and the [Flow](F0001.md#flow--routines));
  3. when `startGame` rejects (back end down), `renderBoard` is **not** called and the rejection surfaces — proves the [Errors](F0001.md#errors) row "render is skipped on a non-2xx" (no partial board).

## TC-UT-F0001-009 — re-render (rematch / new pieces count) reuses the grid and swaps the pieces

- **Given:** a jsdom `#board` already rendered once with `buildBoard(STANDARD)` (64 squares + 32 pieces present on screen). The player then triggers a rematch with `mode=random&pieces=4`, yielding the `RANDOM4` fixture (8 pieces).
- **Input:** a second call `renderBoard(#board, buildBoard(RANDOM4))` into the **same** root.
- **When:** the renderer re-renders into the already-built grid.
- **Then:**
  1. `#board` still has exactly 64 `[data-square]` elements — proves **F-4** (the grid is reused, not rebuilt or duplicated; not 128 squares);
  2. the piece set now deep-equals `cells(RANDOM4)` — exactly 8 `[data-piece]` elements (4 white + 4 black, one king each) on the right squares — proves the new `pieces` count renders correctly (**F-5**) and that the caller-set count, not a random one, drives the result (**B-3**);
  3. none of the STANDARD pieces remain (e.g. `[data-square="a8"]` no longer holds a piece) — proves the previous position is fully cleared on re-render;
  4. each `[data-piece]` is still nested inside its `[data-square]` — proves **F-4** holds across re-renders too.

# Running locally

Add Vitest + jsdom to the front (`cd chessgame && npm i -D vitest jsdom @vitest/ui`), set `test.environment = "jsdom"` in `vite.config.ts`, and add `"test": "vitest"` to `package.json`. Run the suite with:

```
cd chessgame
npm run test          # watch mode
npm run test -- run   # one-shot, CI-style
```

These tests need **no** running `fisher-server`: `domain`/`renderBoard` are pure or DOM-only, and `startGame`'s `fetch` is mocked (TC-007, TC-008).

# Test file

- File: [`chessgame/src/tests/unit_test_F0001.test.ts`](../../../chessgame/src/tests/unit_test_F0001.test.ts).
- Suite: `describe('UT-F0001 — start a game (front render)', …)`.
- TC ↔ test mapping (Vitest `test()` names, numbered by tens to mirror the IT convention): `TC-UT-F0001-001 ↔ t10_cells_standard_32_squares`, `-002 ↔ t20_cells_sparse_overview`, `-003 ↔ t30_square_colour_parity`, `-004 ↔ t40_board_model_64_cells`, `-005 ↔ t50_render_grid_then_pieces`, `-006 ↔ t60_edge_labels`, `-007 ↔ t70_infra_start_game_parse_and_seam`, `-008 ↔ t80_on_page_load_orchestration`, `-009 ↔ t90_render_rerender_reuses_grid`. Each `test()` restates its Given/When/Then in a leading comment, and every `expect(...)` message ties the outcome to the rule it proves.
