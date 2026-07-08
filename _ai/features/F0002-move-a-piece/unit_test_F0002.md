
```yaml
id: UT-F0002
title: Front unit tests — move a piece by two clicks, redraw from the returned board
type: unit-test
status: draft
target_stream: GAME-PLAY
related_feature: F0002
naming_prefix: UT-F0002
language: typescript
```

# Coverage goal

Local, fast validation of the **front-end** move logic at the **module boundary**, with no real back end and no real browser: piece classification, the two-click selection state machine, the network seam, and the smart redraw. The tests run under **Vitest (jsdom)** and call the front's own `domain`, `infra`, and `events` routines directly. Vitest mocks stand in for `movePiece`'s `fetch`, and for the routine itself in the state-machine cases. The board model is the project's own `Overview`, **not** FEN. The `Overview` cell iterator and render helpers are already covered by [UT-F0001](../F0001-start-a-game/unit_test_F0001.md); this spec reuses them and does not re-prove them.

What is asserted:

- `isWhitePiece` returns `true` for an uppercase piece letter and `false` for lowercase or empty — proves the classifier that backs the `square1` front check (**F-1**);
- a first click on a white piece sets `square1` and highlights it — proves **F-1**;
- a first click on an empty or black square selects nothing and sends no request — proves **F-2**;
- clicking the held `square1` again clears the selection and its highlight — proves **F-3**;
- a second click on a different square calls `movePiece` once with the retained `uuid` and the two squares — proves **F-4**;
- on a `valid` response the front hands the returned `overview` to the smart redraw and keeps both squares highlighted — proves **F-5**;
- on an `illegal` response the front shows the reason and clears the selection and highlights — proves **F-6**;
- `movePiece` posts the move, resolves `200` to a `valid` outcome and `422` to an `illegal` `{ reason }`, and throws on any other status — proves **F-7**;
- the smart redraw patches only the squares whose piece changed and leaves every unchanged square's DOM node untouched — proves **F-5**;
- a first click on a fresh source resets every square a previous move left highlighted, leaving only the new source coloured — proves **F-9**.

The rules referenced above are defined in [F0002.md → Rules](F0002.md#rules).

# System under test

## The routines under test

The named front routines from [F0002.md → Flow & routines](F0002.md#flow--routines), called directly — the boundary is the function, not the public REST API.

| Routine | What the tests drive | Cases |
|---------|----------------------|-------|
| `domain/pieces.ts` — `isWhitePiece(letter)` | Pure classifier: `true` for a non-empty uppercase piece letter (`P N B R Q K`), `false` for a lowercase (black) letter or `""`. Backs the `square1` front check (**F-1**). | TC-001 |
| `events/selection.ts` — `initSelection(uuid)` / `onSquareClick(square)` | The click state machine: `initSelection` sets the active game `uuid` and clears any selection (the seam `onPageLoad` uses to retain the `uuid`, **F-8**); `onSquareClick` reads each clicked square's piece from the rendered DOM, classifies it with `isWhitePiece`, owns the `selected` highlight, resets a prior move's highlights on a fresh source (**F-9**), and on the second click drives `movePiece`, then `applyValidMove` (valid) or `rejectMove` (illegal). | TC-002–007, TC-012 |
| `infra/movePiece.ts` — `movePiece(req)` | The single network seam for moves (**F-7**): `POST /move-a-piece`, mapping `200` → a `valid` outcome and `422` → an `illegal` `{ reason }`, throwing on any other status. | TC-008–010 |
| `events/applyValidMove.ts` — `applyValidMove(overview)` | The **smart redraw** (**F-5**): rebuild the model with `buildBoard(overview)` and patch **only** the squares whose piece differs from what is rendered, leaving unchanged squares and the `selected` highlights in place. | TC-006, TC-011 |
| `events/rejectMove.ts` — `rejectMove(message)` | Write the message to the page's `#message` element and remove the `selected` highlights (**F-6**). | TC-007 |

## Harness

Vitest (jsdom). Each selection or redraw case first puts the board on screen with the F0001 helpers — `renderBoard(root, buildBoard(<fixture>))` — already proven by [UT-F0001](../F0001-start-a-game/unit_test_F0001.md). The network is always mocked. The `movePiece` cases replace `globalThis.fetch` with a Vitest mock (`vi.fn`); the state-machine cases replace `infra.movePiece` itself with a spy, so the test sees the call arguments and the branch taken rather than the network. **The back end is never contacted** — per [coding-rules.md](../../global/coding-rules.md) §1 front tests mock `infra`'s network.

## Boundary

`domain` is **pure** — call it and inspect the return value. The `events` routines run against a jsdom `document`; assertions read the produced DOM (`classList`, `dataset`, `querySelector`), **never internal variables**. No socket is opened and no real server runs.

The move seam's request fields:

| Field  | Wire form         | Meaning                                              |
|--------|-------------------|-----------------------------------------------------|
| `uuid` | string (v4)       | The game to move in — retained from F0001 (**F-8**). |
| `from` | string `a1..h8`   | `square1`, the first click; must hold a white piece. |
| `to`   | string `a1..h8`   | `square2`, the second click; differs from `from`.    |

The seam resolves one of two outcomes (the status field carries the verdict):

| Outcome  | Fields                                                                                     |
|----------|--------------------------------------------------------------------------------------------|
| `valid`   | `from`, `to`, `piece` (moved piece), `capture` (taken black letter or none), `overview` (the board after the move). |
| `illegal` | `reason` — one of the closed set from [F0002.md → Output](F0002.md#output).                 |

## Fixtures

Four `Overview` boards are reused across cases; each is shown once below and referenced by name. Uppercase is White, lowercase is Black, `.` is empty, `board[0]` is rank 8. Castling fields are `white = "both"`, `black = "both"` for all four.

**STANDARD** — the standard opening (the selection cases click on it):

```
   a b c d e f g h
 8 r n b q k b n r
 7 p p p p p p p p
 6 . . . . . . . .
 5 . . . . . . . .
 4 . . . . . . . .
 3 . . . . . . . .
 2 P P P P P P P P
 1 R N B Q K B N R
```

**AFTER_D2D4** — STANDARD after the pawn push `d2` → `d4` (`d2` emptied, `d4` filled):

```
   a b c d e f g h
 8 r n b q k b n r
 7 p p p p p p p p
 6 . . . . . . . .
 5 . . . . . . . .
 4 . . . P . . . .
 3 . . . . . . . .
 2 P P P . P P P P
 1 R N B Q K B N R
```

**CAPTURE_BEFORE** — a capture position (White `P` `e4`, Black `p` `d5`, kings `e1`/`e8`):

```
   a b c d e f g h
 8 . . . . k . . .
 7 . . . . . . . .
 6 . . . . . . . .
 5 . . . p . . . .
 4 . . . . P . . .
 3 . . . . . . . .
 2 . . . . . . . .
 1 . . . . K . . .
```

**CAPTURE_AFTER** — CAPTURE_BEFORE after `e4` → `d5` (the pawn takes and lands on `d5`, `e4` emptied):

```
   a b c d e f g h
 8 . . . . k . . .
 7 . . . . . . . .
 6 . . . . . . . .
 5 . . . P . . . .
 4 . . . . . . . .
 3 . . . . . . . .
 2 . . . . . . . .
 1 . . . . K . . .
```

# Test cases

## TC-UT-F0002-001 — isWhitePiece classifies by case

- **Given:** no fixture; pure letter input.

  | Letter | Expected | Reason                          |
  |--------|----------|---------------------------------|
  | `"P"`  | `true`   | uppercase white piece           |
  | `"K"`  | `true`   | uppercase white piece           |
  | `"p"`  | `false`  | lowercase black piece           |
  | `"k"`  | `false`  | lowercase black piece           |
  | `""`   | `false`  | empty square — nothing to move  |

- **Input:** `isWhitePiece(letter)` for each row.
- **When:** the classifier reads the letter's case.
- **Then:**
  1. each call returns the Expected value — proves the classifier that backs the `square1` front check accepts only a white piece as a source (**F-1**), so an empty or black square is refused before any selection (**F-2**).

## TC-UT-F0002-002 — first click on a white piece selects and highlights it

- **Given:** a jsdom `#board` rendered from STANDARD via `renderBoard(root, buildBoard(STANDARD))`, `initSelection("u-1")`, and `infra.movePiece` replaced by a spy.
- **Input:** `onSquareClick("d2")` — `d2` holds a white pawn.
- **When:** the state machine handles the first click.
- **Then:**
  1. the `[data-square="d2"]` element carries the `selected` class — proves **F-1** (the white source is selected and highlighted);
  2. no other square carries `selected` — proves only `square1` is held;
  3. the `movePiece` spy was **not** called — proves the first click never sends a request (**F-1**/**F-4**).

## TC-UT-F0002-003 — first click on an empty or black square selects nothing

- **Given:** a jsdom `#board` rendered from STANDARD, `initSelection("u-1")`, and `infra.movePiece` spied.

  | Click on | Square content        | Expected                                        |
  |----------|-----------------------|-------------------------------------------------|
  | `e4`     | empty in the opening  | no selection, no request                        |
  | `d7`     | black pawn `"p"`      | no selection, no request — White only (**F-2**) |

- **Input:** `onSquareClick("e4")`, then (fresh) `onSquareClick("d7")`.
- **When:** the state machine handles a first click on a non-white square.
- **Then:**
  1. no `[data-square]` element carries `selected` — proves **F-2** (an empty or black source selects nothing);
  2. the `movePiece` spy was not called — proves the front sends no request for an invalid source (**F-2**).

## TC-UT-F0002-004 — re-clicking the source deselects it

- **Given:** a jsdom `#board` rendered from STANDARD, `initSelection("u-1")`, `infra.movePiece` spied, and a first click already made on `d2` (so `d2` is `selected`).
- **Input:** a second `onSquareClick("d2")` — the same square again.
- **When:** the state machine sees the held `square1` re-clicked.
- **Then:**
  1. `[data-square="d2"]` no longer carries `selected` — proves **F-3** (re-click cancels the selection and removes its highlight);
  2. no `[data-square]` carries `selected` — proves the machine returned to the no-selection state;
  3. the `movePiece` spy was not called — proves a deselect sends no move (**F-3**).

## TC-UT-F0002-005 — second click on a different square sends the move

- **Given:** a jsdom `#board` rendered from STANDARD, `initSelection("u-1")`, `infra.movePiece` spied to resolve a `valid` outcome, and a first click already made on `d2`.
- **Input:** `onSquareClick("d4")` — a different square.
- **When:** the state machine sees `square1` held and a second, different square clicked.
- **Then:**
  1. `movePiece` was called exactly once with `uuid` `"u-1"`, `from` `"d2"`, `to` `"d4"` — proves **F-4** (the second click sends the retained `uuid` and the two squares, with no confirmation step);
  2. at the moment of the call both `[data-square="d2"]` and `[data-square="d4"]` carry `selected` — proves the target is highlighted before the request goes out (**F-4**).

## TC-UT-F0002-006 — a valid response redraws from the returned board and keeps highlights

- **Given:** a jsdom `#board` rendered from STANDARD, `initSelection("u-1")`, `infra.movePiece` mocked to resolve a `valid` outcome for `from` `"d2"`, `to` `"d4"`, `piece` `"P"`, no capture, and `overview` AFTER_D2D4, and `events.applyValidMove` spied.
- **Input:** two clicks — `onSquareClick("d2")` then `onSquareClick("d4")` — awaiting the settled `movePiece` promise.
- **When:** the `valid` branch runs.
- **Then:**
  1. `applyValidMove` was called exactly once with the response's `overview` (AFTER_D2D4) — proves **F-5** (a valid move is applied by redrawing from the returned board, not by hand-moving pieces);
  2. `rejectMove` was not called — proves the valid and illegal branches are exclusive;
  3. both `[data-square="d2"]` and `[data-square="d4"]` still carry `selected` — proves the redraw leaves both squares highlighted (**F-5**).

## TC-UT-F0002-007 — an illegal response shows the reason and clears the selection

- **Given:** a jsdom `#board` rendered from STANDARD with a `#message` element present, `initSelection("u-1")`, and `infra.movePiece` mocked to resolve an `illegal` outcome with `reason` `"path blocked"`.
- **Input:** two clicks — `onSquareClick("d1")` then `onSquareClick("d3")` — awaiting the settled `movePiece` promise.
- **When:** the `illegal` branch runs.
- **Then:**
  1. the `#message` element's text carries `"path blocked"` — proves **F-6** (the reason is shown on the page);
  2. no `[data-square]` carries `selected` — proves the selection and both highlights are cleared (**F-6**);
  3. the rendered pieces are unchanged from STANDARD (the board is left as it was) — proves an illegal move applies nothing on the front (**F-6**).

## TC-UT-F0002-008 — movePiece posts the move and resolves a valid outcome through one seam

- **Given:** `globalThis.fetch` replaced by a Vitest mock that resolves an `ok`, status-`200` response whose JSON body carries `from` `"d2"`, `to` `"d4"`, `piece` `"P"`, no capture, and `overview` AFTER_D2D4.
- **Input:** `await movePiece({ uuid: "u-1", from: "d2", to: "d4" })`.
- **When:** `infra` builds the request and parses the JSON body.
- **Then:**
  1. the resolved value is a `valid` outcome carrying that `from`/`to`/`piece`/`capture`/`overview` (the F0002 [Output](F0002.md#output) shape, including the nested `overview`) — proves `infra` maps `200` to a `valid` outcome;
  2. `fetch` was called once, with a `POST` to a URL containing `/move-a-piece` and a JSON body deep-equal to the request `uuid`/`from`/`to` — proves **F-7** (the single network seam sends the move contract);
  3. no DOM node is created by `infra` — proves the role split (`infra` ≠ render) from [coding-rules.md](../../global/coding-rules.md) §1.

## TC-UT-F0002-009 — movePiece maps a 422 to an illegal outcome

- **Given:** `globalThis.fetch` replaced by a mock resolving a not-`ok`, status-`422` response whose JSON body carries `reason` `"king in check"`.
- **Input:** `await movePiece({ uuid: "u-1", from: "e2", to: "d3" })`.
- **When:** `infra` reads the `422` response.
- **Then:**
  1. the resolved value is an `illegal` outcome with `reason` `"king in check"` — proves **F-7** (a `422` is a normal illegal verdict, resolved rather than thrown, carrying the closed-set `reason`).

## TC-UT-F0002-010 — movePiece throws on any other status

- **Given:** `globalThis.fetch` replaced per row.

  | fetch resolves / rejects                            | Expected            | Reason                                          |
  |-----------------------------------------------------|---------------------|-------------------------------------------------|
  | not-`ok`, status `404` (`"unknown game"`)           | `movePiece` throws  | `404` is a real error, not a verdict (**F-7**)  |
  | not-`ok`, status `400` (`"invalid move request"`)   | `movePiece` throws  | `400` is a real error, not a verdict (**F-7**)  |
  | a rejected promise (network down)                   | `movePiece` throws  | a transport failure is a real error (**F-7**)   |

- **Input:** `await movePiece({ uuid: "u-1", from: "d2", to: "d4" })` for each row.
- **When:** `infra` sees a non-`200`/non-`422` outcome.
- **Then:**
  1. the returned promise rejects (the `await` throws) for every row — proves **F-7** (only `200` and `422` are resolved; `400`/`404`/network surface to `events` as real errors, never as an illegal verdict).

## TC-UT-F0002-011 — the smart redraw patches only the changed squares

- **Given:** a jsdom `#board` rendered from CAPTURE_BEFORE (White `P` on `e4`, Black `p` on `d5`, kings on `e1`/`e8`). Before the redraw, the test captures the exact piece DOM nodes on the squares that do not change.

  | Square | Before | After (CAPTURE_AFTER) | Expected                                    |
  |--------|--------|-----------------------|---------------------------------------------|
  | `e4`   | `P`    | empty                 | piece node removed                          |
  | `d5`   | `p`    | `P`                   | node replaced — old `p` gone, new `P` there |
  | `e1`   | `K`    | `K`                   | **same** DOM node — untouched               |
  | `e8`   | `k`    | `k`                   | **same** DOM node — untouched               |

- **Input:** `applyValidMove(CAPTURE_AFTER)`.
- **When:** the smart redraw diffs the model from CAPTURE_AFTER against the rendered board and patches the differences.
- **Then:**
  1. `[data-square="e4"]` holds no `[data-piece]` and `[data-square="d5"]` holds a `[data-piece]` whose `dataset.piece` is `"P"` — proves **F-5** (the moved pawn lands on the target and overwrites the taken piece, so the capture needs no special case);
  2. the `[data-piece]` nodes on `e1` and `e8` are the **same** element references captured before the redraw — proves **F-5** (only changed squares are patched; unchanged squares' DOM is left in place);
  3. `#board` still has exactly 64 `[data-square]` elements — proves the grid is never rebuilt (**F-5**, and F0001 **F-4** carried forward).

## TC-UT-F0002-012 — a new source selection resets the previous move's highlights

- **Given:** a jsdom `#board` rendered from STANDARD, `initSelection("u-1")`, and `infra.movePiece` spied. Two squares are pre-marked as if a previous move left them highlighted — `d2` as the old source and `d4` as the old target (each carrying the `selected` highlight, as rule **F-5** leaves them).
- **Input:** `onSquareClick("e2")` — a first click on a fresh white source, with no `square1` held.
- **When:** the state machine handles the first click on a new source.
- **Then:**
  1. neither `[data-square="d2"]` nor `[data-square="d4"]` carries `selected` — proves **F-9** (every square a previous move left highlighted is reset before the new source is coloured);
  2. `[data-square="e2"]` carries `selected` and it is the only highlighted square — proves **F-9**/**F-1** (only the current selection is ever highlighted);
  3. the `movePiece` spy was not called — proves a first click sends no request (**F-1**/**F-4**), kept here for traceability.

# Running locally

Vitest and jsdom are already configured (`vitest.config.ts` sets `environment: "jsdom"` and includes `src/tests/**/*.test.ts`). Run the suite from the front folder:

```
cd chessgame
npm run test          # watch mode
npm run test -- run   # one-shot, CI-style
```

These tests need **no** running `fisher-server`: `isWhitePiece` and `applyValidMove` are pure or DOM-only, and `movePiece`'s `fetch` is mocked (TC-008–TC-010). They exercise the F0002 front routines — `isWhitePiece`, `movePiece`, `onSquareClick`/`initSelection`, `applyValidMove`, and `rejectMove` — which must exist for the suite to resolve; the spec defines the contract the implementation satisfies.

# Test file

- File: [`chessgame/src/tests/unit_test_F0002.test.ts`](../../../chessgame/src/tests/unit_test_F0002.test.ts).
- Suite: `describe('UT-F0002 — move a piece (front)', …)`.
- TC ↔ test mapping (Vitest `test()` names, numbered by tens to mirror the IT convention): `TC-UT-F0002-001 ↔ t10_is_white_piece`, `-002 ↔ t20_first_click_selects_white`, `-003 ↔ t30_first_click_empty_or_black_refused`, `-004 ↔ t40_reclick_deselects`, `-005 ↔ t50_second_click_sends_move`, `-006 ↔ t60_valid_applies_and_keeps_highlights`, `-007 ↔ t70_illegal_shows_reason_and_clears`, `-008 ↔ t80_move_piece_valid_and_seam`, `-009 ↔ t90_move_piece_illegal_422`, `-010 ↔ t100_move_piece_throws_other_status`, `-011 ↔ t110_smart_redraw_patches_changed_only`, `-012 ↔ t120_new_selection_resets_old_highlights`. Each `test()` restates its Given/When/Then in a leading comment, and every `expect(...)` message ties the outcome to the rule it proves.
