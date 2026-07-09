
```yaml
id: UT-F0003
title: Front unit tests — trigger Stockfish's reply, redraw from the returned board, border and log the move
type: unit-test
status: draft
target_stream: GAME-PLAY
related_feature: F0003
naming_prefix: UT-F0003
language: typescript
```

# Coverage goal

Local, fast validation of the **front-end** opponent-reply logic at the **module boundary**, with no real back end and no real browser: the auto-trigger after a valid White move, the network seam, the smart redraw from the returned board, the red/blue borders on Black's move, and the log/status line. The tests run under **Vitest (jsdom)** and call the front's own `infra` and `events` routines directly. Vitest mocks stand in for `opponentMove`'s `fetch`, and for the routines themselves (`infra.opponentMove`, `events.playOpponent`, `events.applyValidMove`) in the orchestration and trigger cases. The board model is the project's own `Overview`, **not** FEN. The `Overview` cell iterator, the render helpers, and the two-click selection machine are already covered by [UT-F0001](../F0001-start-a-game/unit_test_F0001.md) and [UT-F0002](../F0002-move-a-piece/unit_test_F0002.md); this spec reuses them and does not re-prove them.

What is asserted:

- as soon as a valid White move is applied, the front calls `playOpponent` with the retained game `uuid` — proves the auto-trigger (**F-1**);
- `opponentMove` posts `{ uuid }`, resolves `200` into the full `OpponentMoveResponse`, and throws on any other status — proves the single network seam (**F-5**);
- `playOpponent` hands a resolved reply to `applyOpponentMove`, and on a thrown engine failure logs an engine-error and leaves the board untouched — proves **F-5**, **F-6**;
- on a reply that moves, `applyOpponentMove` redraws from the returned `overview`, borders the departure square red and the arrival square blue, and appends a log entry with the move and status — proves **F-2**, **F-3**, **F-4**;
- the smart redraw patches only the squares whose piece changed and leaves every unchanged square's DOM node in place — proves **F-2**;
- on a game-over reply (a `null` move — `black-in-checkmate`/`black-in-stalemate`/`white-in-checkmate`), `applyOpponentMove` redraws nothing and sets no borders, but shows the terminal status — proves **F-2**, **F-4**;
- a fresh White source click clears the black-reply borders left by the last reply — proves **F-3** (carrying F0002 **F-9**).

The rules referenced above are defined in [F0003.md → Rules](F0003.md#rules); the reply shape and the game-status set are in [F0003.md → Output](F0003.md#output).

# System under test

## The routines under test

The named front routines from [F0003.md → Flow & routines](F0003.md#flow--routines), called directly — the boundary is the function, not the public REST API.

| Routine | What the tests drive | Cases |
|---------|----------------------|-------|
| `events/selection.ts` — `onSquareClick(square)` | Extended from F0002: on a valid White move the valid branch triggers `playOpponent` with the retained `uuid` (**F-1**), and a fresh White source click resets the black-reply borders (**F-3**, carrying F0002 **F-9**). | TC-001, TC-008 |
| `infra/opponentMove.ts` — `opponentMove(req)` | The single network seam for the reply (**F-5**): `POST /opponent-move` with `{ uuid }`, resolving `200` into an `OpponentMoveResponse` and throwing on any other status. | TC-002, TC-003 |
| `events/playOpponent.ts` — `playOpponent(uuid)` | Orchestrates the reply: calls `infra.opponentMove({ uuid })`, then `applyOpponentMove` on success, or logs an engine-error and leaves the board on a thrown failure (**F-5**, **F-6**). | TC-004 |
| `events/applyOpponentMove.ts` — `applyOpponentMove(resp)` | DOM writer: **smart redraw** from `resp.overview` (nothing to redraw when the move is `null`), a red border on `resp.from` and a blue border on `resp.to`, and a log entry carrying the move and `resp.status` (**F-2**, **F-3**, **F-4**). | TC-005, TC-006, TC-007 |

## Harness

Vitest (jsdom). Each redraw, border, or trigger case first puts the board on screen with the F0001 helpers — `renderBoard(root, buildBoard(<fixture>))` — already proven by [UT-F0001](../F0001-start-a-game/unit_test_F0001.md), and adds a `#log-list` element so the log/status line has somewhere to land (the log panel is a no-op when that element is absent). The network is always mocked. The `opponentMove` cases replace `globalThis.fetch` with a Vitest mock (`vi.fn`); the trigger and orchestration cases replace `infra.opponentMove`, `events.playOpponent`, and `events.applyValidMove` with spies, so the test sees the call arguments and the branch taken rather than the network. **The back end is never contacted** — per [coding-rules.md](../../global/coding-rules.md) §1 front tests mock `infra`'s network.

## Boundary

`infra` builds the request and parses the JSON; assertions read its resolved value and the mock's calls, never a socket. The `events` routines run against a jsdom `document`; assertions read the produced DOM (`classList`, `dataset`, `querySelector`, the `#log-list` entries), **never internal variables**. No real server runs.

The reply seam's request field:

| Field  | Wire form   | Meaning                                    |
|--------|-------------|--------------------------------------------|
| `uuid` | string (v4) | The game to answer in — retained from F0001 (**F-8**). |

The seam resolves one `OpponentMoveResponse` (the `status` field carries the verdict) or throws:

| Field      | Wire form                     | Meaning                                                        |
|------------|-------------------------------|---------------------------------------------------------------|
| `from`     | string `a1..h8` \| `null`     | Black's source square; `null` when the game was already over. |
| `to`       | string `a1..h8` \| `null`     | Black's target square; `null` likewise.                       |
| `piece`    | string (lowercase) \| `null`  | The black piece that moved; `null` likewise.                  |
| `capture`  | string (uppercase) \| `null`  | The captured white piece, or `null` when the reply takes nothing. |
| `status`   | `GameStatus` string           | `move` / `white-in-check` / `white-in-checkmate` / `black-in-checkmate` / `black-in-stalemate`. |
| `overview` | `Overview`                    | The board after Black's move — or unchanged when the game was already over. |

## Fixtures

Five `Overview` boards are reused across cases; each is shown once below and referenced by name. Uppercase is White, lowercase is Black, `.` is empty, `board[0]` is rank 8. Castling fields are `white = "both"`, `black = "both"` for all five.

**STANDARD** — the standard opening (the trigger and border-reset cases click on it):

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

**E4_BEFORE** — the opening after `1.e4` (White pawn on `e4`), Black to move — the board rendered before Black's reply:

```
   a b c d e f g h
 8 r n b q k b n r
 7 p p p p p p p p
 6 . . . . . . . .
 5 . . . . . . . .
 4 . . . . P . . .
 3 . . . . . . . .
 2 P P P P . P P P
 1 R N B Q K B N R
```

**E4_AFTER_C5** — E4_BEFORE after Black plays `c7` → `c5` (the returned `overview` for the move case):

```
   a b c d e f g h
 8 r n b q k b n r
 7 p p . p p p p p
 6 . . . . . . . .
 5 . . p . . . . .
 4 . . . . P . . .
 3 . . . . . . . .
 2 P P P P . P P P
 1 R N B Q K B N R
```

**CAP_BEFORE** — a Black-captures position (White `K` `e1`, White `Q` `d5`, Black `K` `e8`, Black `p` `e6`); Black's reply `e6` → `d5` takes the queen:

```
   a b c d e f g h
 8 . . . . k . . .
 7 . . . . . . . .
 6 . . . . p . . .
 5 . . . Q . . . .
 4 . . . . . . . .
 3 . . . . . . . .
 2 . . . . . . . .
 1 . . . . K . . .
```

**CAP_AFTER** — CAP_BEFORE after `e6` → `d5` (the black pawn takes and lands on `d5`, overwriting the white queen; `e6` emptied):

```
   a b c d e f g h
 8 . . . . k . . .
 7 . . . . . . . .
 6 . . . . . . . .
 5 . . . p . . . .
 4 . . . . . . . .
 3 . . . . . . . .
 2 . . . . . . . .
 1 . . . . K . . .
```

# Test cases

## TC-UT-F0003-001 — a valid White move triggers the opponent reply

- **Given:** a jsdom `#board` rendered from STANDARD, `initSelection("u-1")`, `infra.movePiece` mocked to resolve a `valid` outcome for `d2` → `d4`, and both `events.applyValidMove` and `events.playOpponent` replaced by spies. A first click is already made on `d2`.
- **Input:** `onSquareClick("d4")` — the second click — awaiting the settled `movePiece` promise.
- **When:** the F0002 valid branch runs and the F0003 trigger fires.
- **Then:**
  1. `playOpponent` was called exactly once with `uuid` `"u-1"` — proves **F-1** (the front asks for Black's reply as soon as a valid White move is applied, with the retained game id);
  2. `applyValidMove` was called before `playOpponent` — proves the trigger fires **after** White's move is drawn, not before (**F-1**);
  3. `playOpponent` was not called on the earlier illegal/first-click paths — proves the reply is triggered only by a valid White move (**F-1**), kept here for traceability.

## TC-UT-F0003-002 — opponentMove posts the game id and resolves the reply through one seam

- **Given:** `globalThis.fetch` replaced by a Vitest mock that resolves an `ok`, status-`200` response whose JSON body is a move reply — `from` `"c7"`, `to` `"c5"`, `piece` `"p"`, `capture` `null`, `status` `"move"`, `overview` E4_AFTER_C5.
- **Input:** `await opponentMove({ uuid: "u-1" })`.
- **When:** `infra` builds the request and parses the JSON body.
- **Then:**
  1. the resolved value carries `from`/`to`/`piece`/`capture`/`status`/`overview` from the body (the F0003 [Output](F0003.md#output) shape, including the nested `overview`) — proves `infra` maps `200` to the full `OpponentMoveResponse`;
  2. `fetch` was called once, with a `POST` to a URL containing `/opponent-move` and a JSON body deep-equal to `{ uuid: "u-1" }` — proves **F-5** (the single network seam sends only the game id);
  3. no DOM node is created by `infra` — proves the role split (`infra` ≠ render) from [coding-rules.md](../../global/coding-rules.md) §1.

## TC-UT-F0003-003 — opponentMove throws on any status other than 200

- **Given:** `globalThis.fetch` replaced per row.

  | fetch resolves / rejects                              | Expected              | Reason                                          |
  |-------------------------------------------------------|-----------------------|-------------------------------------------------|
  | not-`ok`, status `502` (`"engine unavailable"`)       | `opponentMove` throws | the engine is down — a real error, not a verdict (**F-5**, **F-6**) |
  | not-`ok`, status `404` (`"unknown game"`)             | `opponentMove` throws | `404` is a real error (**F-5**)                 |
  | not-`ok`, status `400` (`"invalid opponent-move request"`) | `opponentMove` throws | `400` is a real error (**F-5**)             |
  | a rejected promise (network down)                     | `opponentMove` throws | a transport failure is a real error (**F-5**)   |

- **Input:** `await opponentMove({ uuid: "u-1" })` for each row.
- **When:** `infra` sees a non-`200` outcome.
- **Then:**
  1. the returned promise rejects (the `await` throws) for every row — proves **F-5** (only `200` is resolved; `400`/`404`/`502`/network surface to `events` as real errors, so the engine-failure path is reachable, **F-6**).

## TC-UT-F0003-004 — playOpponent applies a resolved reply and logs an engine failure

- **Given:** a jsdom `#board` rendered from E4_BEFORE with a `#log-list` element present, and `events.applyOpponentMove` spied.

  | `infra.opponentMove` mock         | Expected                                                                 |
  |-----------------------------------|--------------------------------------------------------------------------|
  | resolves a move reply (overview E4_AFTER_C5) | `applyOpponentMove` called once with that reply — the reply is reflected (**F-5**) |
  | throws (engine failure)           | `applyOpponentMove` **not** called; a `#log-list` error entry appears; the board is unchanged (**F-6**) |

- **Input:** `await playOpponent("u-1")` for each row.
- **When:** `playOpponent` awaits the seam and branches on success vs. throw.
- **Then:**
  1. on the resolving row, `applyOpponentMove` was called exactly once with the resolved reply — proves **F-5** (`playOpponent` consumes the seam and hands the reply to the DOM writer);
  2. on the throwing row, `applyOpponentMove` was not called and a new `#log-list` entry (error style) reflects the engine failure — proves **F-6** (an engine failure shows a message, not a partial reply);
  3. on the throwing row, the rendered pieces are unchanged from E4_BEFORE — proves the board and highlights are left as they were on failure (**F-6**).

## TC-UT-F0003-005 — a move reply redraws from the board, borders the move, and logs it

- **Given:** a jsdom `#board` rendered from E4_BEFORE with a `#log-list` element present, and the reply `from` `"c7"`, `to` `"c5"`, `piece` `"p"`, `capture` `null`, `status` `"move"`, `overview` E4_AFTER_C5.
- **Input:** `applyOpponentMove(reply)`.
- **When:** the DOM writer redraws from the returned board and marks the move.
- **Then:**
  1. in the rendered board, `cell(ov, "c7") == ""` and `cell(ov, "c5")` holds a `[data-piece]` whose `dataset.piece` is `"p"` — proves **F-2** (the reply is applied by redrawing from `resp.overview`, not by hand-moving);
  2. `[data-square="c7"]` carries the `last-move-from` (red) class and `[data-square="c5"]` carries the `last-move-to` (blue) class — proves **F-3** (the departure square is red, the arrival square is blue);
  3. exactly those two squares carry a black-reply border, and no `selected` highlight is added — proves the borders are the reply's own marking, distinct from the White-selection highlight (**F-3**);
  4. `#log-list` gains one entry whose text carries the move (`c7`→`c5`) and the `status` (`move`) — proves **F-4** (the move and status are shown in the log zone).

## TC-UT-F0003-006 — the smart redraw patches only the changed squares

- **Given:** a jsdom `#board` rendered from CAP_BEFORE (White `Q` on `d5`, Black `p` on `e6`, kings on `e1`/`e8`). Before the redraw, the test captures the exact piece DOM nodes on the squares that do not change. The reply is `from` `"e6"`, `to` `"d5"`, `piece` `"p"`, `capture` `"Q"`, `status` `"move"`, `overview` CAP_AFTER.

  | Square | Before | After (CAP_AFTER) | Expected                                    |
  |--------|--------|-------------------|---------------------------------------------|
  | `e6`   | `p`    | empty             | piece node removed                          |
  | `d5`   | `Q`    | `p`               | node replaced — old `Q` gone, new `p` there |
  | `e1`   | `K`    | `K`               | **same** DOM node — untouched               |
  | `e8`   | `k`    | `k`               | **same** DOM node — untouched               |

- **Input:** `applyOpponentMove(reply)`.
- **When:** the smart redraw diffs the model from CAP_AFTER against the rendered board and patches the differences.
- **Then:**
  1. `[data-square="e6"]` holds no `[data-piece]` and `[data-square="d5"]` holds a `[data-piece]` whose `dataset.piece` is `"p"` — proves **F-2** (the black pawn lands on the target and overwrites the taken queen, so a capture needs no special case);
  2. the `[data-piece]` nodes on `e1` and `e8` are the **same** element references captured before the redraw — proves **F-2** (only changed squares are patched; unchanged squares' DOM is left in place);
  3. `#board` still has exactly 64 `[data-square]` elements — proves the grid is never rebuilt (**F-2**, and F0001 **F-4** carried forward).

## TC-UT-F0003-007 — a game-over reply redraws nothing but shows the status

- **Given:** a jsdom `#board` rendered from CAP_BEFORE with a `#log-list` element present, and the test captures the current piece DOM nodes. The reply is a game-over one — `from` `null`, `to` `null`, `piece` `null`, `capture` `null`, `status` `"black-in-checkmate"`, `overview` CAP_BEFORE (the board is unchanged).
- **Input:** `applyOpponentMove(reply)`.
- **When:** the DOM writer sees a `null` move.
- **Then:**
  1. no `[data-square]` carries `last-move-from` or `last-move-to` — proves **F-3** (there is no move to border);
  2. the captured piece DOM nodes are the same element references after the call — proves **F-2** (nothing is redrawn when the move is `null` and the board is unchanged);
  3. `#log-list` gains one entry reflecting the terminal `status` (`black-in-checkmate`) — proves **F-4** (the status is shown and the game is over, so no further White move is expected).

## TC-UT-F0003-008 — a fresh White source click clears the black-reply borders

- **Given:** a jsdom `#board` rendered from STANDARD, `initSelection("u-1")`, `infra.movePiece` spied, and two squares pre-marked as the last reply's move — `d7` carrying `last-move-from` (red) and `d5` carrying `last-move-to` (blue).
- **Input:** `onSquareClick("d2")` — a first click on a fresh White source, with no `square1` held.
- **When:** the state machine handles the first click on a new source.
- **Then:**
  1. neither `[data-square="d7"]` nor `[data-square="d5"]` carries `last-move-from` or `last-move-to` — proves **F-3** (a new White move clears the black-reply borders, via F0002 **F-9**'s reset extended to those classes);
  2. `[data-square="d2"]` carries `selected` and is the only highlighted square — proves **F-9**/**F-1** (only the current selection is ever highlighted), carried forward from UT-F0002;
  3. the `movePiece` spy was not called — proves a first click sends no request, kept here for traceability.

# Running locally

Vitest and jsdom are already configured (`vitest.config.ts` sets `environment: "jsdom"` and includes `src/tests/**/*.test.ts`). Run the suite from the front folder:

```
cd chessgame
npm run test          # watch mode
npm run test -- run   # one-shot, CI-style
```

These tests need **no** running `fisher-server` and no running Stockfish engine: `applyOpponentMove` is DOM-only, and `opponentMove`'s `fetch` is mocked (TC-002, TC-003). They exercise the F0003 front routines — `opponentMove`, `playOpponent`, `applyOpponentMove`, and the extended `onSquareClick` — which must exist for the suite to resolve; the spec defines the contract the implementation satisfies.

# Test file

- File: [`chessgame/src/tests/unit_test_F0003.test.ts`](../../../chessgame/src/tests/unit_test_F0003.test.ts).
- Suite: `describe('UT-F0003 — play with Stockfish (front)', …)`.
- TC ↔ test mapping (Vitest `test()` names, numbered by tens to mirror the IT convention): `TC-UT-F0003-001 ↔ t10_valid_move_triggers_reply`, `-002 ↔ t20_opponent_move_resolves_200`, `-003 ↔ t30_opponent_move_throws_other_status`, `-004 ↔ t40_play_opponent_applies_or_logs_failure`, `-005 ↔ t50_move_reply_redraws_borders_logs`, `-006 ↔ t60_smart_redraw_patches_changed_only`, `-007 ↔ t70_game_over_reply_shows_status`, `-008 ↔ t80_new_source_clears_reply_borders`. Each `test()` restates its Given/When/Then in a leading comment, and every `expect(...)` message ties the outcome to the rule it proves.
