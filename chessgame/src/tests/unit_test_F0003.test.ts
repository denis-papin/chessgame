// Front unit tests for feature F0003 (UT-F0003) — trigger Stockfish's reply,
// redraw from the returned board, border and log the move. Vitest + jsdom. No
// real back end (fetch / infra mocked) and no real browser. The board model is
// the project's own `Overview`, not FEN. The F0001/F0002 helpers are reused, not
// re-proven.

import { afterEach, describe, expect, test, vi } from 'vitest'

import { buildBoard } from '../domain/overview'
import type { Overview } from '../domain/overview'
import { renderBoard } from '../events/renderBoard'

import { opponentMove } from '../infra/opponentMove'
import * as infraOpp from '../infra/opponentMove'
import type { OpponentMoveResponse } from '../infra/opponentMove'
import { playOpponent } from '../events/playOpponent'
import * as opponentMod from '../events/playOpponent'
import { applyOpponentMove } from '../events/applyOpponentMove'
import * as applyOppMod from '../events/applyOpponentMove'
import * as applyValidMod from '../events/applyValidMove'
import * as infraMove from '../infra/movePiece'
import { initSelection, onSquareClick } from '../events/selection'

// ---- fixtures ---------------------------------------------------------------

const STANDARD: Overview = {
  board: [
    ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
    ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
    ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
  ],
  white: 'both',
  black: 'both',
}

// The opening after 1.e4 — the board rendered before Black's reply.
const E4_BEFORE: Overview = {
  board: [
    ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
    ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', 'P', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['P', 'P', 'P', 'P', '', 'P', 'P', 'P'],
    ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
  ],
  white: 'both',
  black: 'both',
}

// E4_BEFORE after Black plays c7 -> c5 (the returned overview for the move case).
const E4_AFTER_C5: Overview = {
  board: [
    ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
    ['p', 'p', '', 'p', 'p', 'p', 'p', 'p'],
    ['', '', '', '', '', '', '', ''],
    ['', '', 'p', '', '', '', '', ''],
    ['', '', '', '', 'P', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['P', 'P', 'P', 'P', '', 'P', 'P', 'P'],
    ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
  ],
  white: 'both',
  black: 'both',
}

// A Black-captures position: White K e1, White Q d5, Black K e8, Black p e6.
const CAP_BEFORE: Overview = {
  board: [
    ['', '', '', '', 'k', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', 'p', '', '', ''],
    ['', '', '', 'Q', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', 'K', '', '', ''],
  ],
  white: 'both',
  black: 'both',
}

// CAP_BEFORE after e6 -> d5 (the pawn takes the queen and lands on d5).
const CAP_AFTER: Overview = {
  board: [
    ['', '', '', '', 'k', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', 'p', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', 'K', '', '', ''],
  ],
  white: 'both',
  black: 'both',
}

// ---- helpers ----------------------------------------------------------------

/** Render `ov` into a fresh `#board` and return the root. */
function renderInto(ov: Overview): HTMLElement {
  document.body.innerHTML = '<div id="board"></div>'
  const root = document.getElementById('board') as HTMLElement
  renderBoard(root, buildBoard(ov))
  return root
}

/** Render `ov` into `#board` and add a `#log-list` for the log/status line. */
function renderWithLog(ov: Overview): HTMLElement {
  document.body.innerHTML = '<div id="board"></div><ul id="log-list"></ul>'
  const root = document.getElementById('board') as HTMLElement
  renderBoard(root, buildBoard(ov))
  return root
}

const pieceOn = (square: string) =>
  document.querySelector(`[data-square="${square}"] [data-piece]`)?.getAttribute('data-piece') ?? ''

const hasClass = (square: string, cls: string) =>
  document.querySelector(`[data-square="${square}"]`)!.classList.contains(cls)

const replyBorderCount = () =>
  document.querySelectorAll('[data-square].last-move-from, [data-square].last-move-to').length

const logEntries = () => Array.from(document.querySelectorAll('#log-list li')).map((li) => li.textContent ?? '')

/** Flush pending microtasks so a settled promise resolves. */
const flush = () => new Promise<void>((r) => setTimeout(r, 0))

/** Occupied squares as `{ square, piece }`, sorted — for the unchanged-board assertion. */
function renderedPieces(root: HTMLElement) {
  return Array.from(root.querySelectorAll<HTMLElement>('[data-piece]'))
    .map((el) => ({ square: (el.parentElement as HTMLElement).dataset.square, piece: el.dataset.piece }))
    .sort((a, b) => (a.square ?? '').localeCompare(b.square ?? ''))
}

/** A move reply: Black plays c7 -> c5, status `move`. */
const MOVE_REPLY: OpponentMoveResponse = {
  from: 'c7', to: 'c5', piece: 'p', capture: null, status: 'move', overview: E4_AFTER_C5,
}

/** A capture reply: Black plays e6 -> d5 taking the white queen. */
const CAPTURE_REPLY: OpponentMoveResponse = {
  from: 'e6', to: 'd5', piece: 'p', capture: 'Q', status: 'move', overview: CAP_AFTER,
}

/** A game-over reply: Black is checkmated — no move, board unchanged. */
const GAME_OVER_REPLY: OpponentMoveResponse = {
  from: null, to: null, piece: null, capture: null, status: 'black-in-checkmate', overview: CAP_BEFORE,
}

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  document.body.innerHTML = ''
  initSelection('') // reset the selection module's state between tests
})

describe('UT-F0003 — play with Stockfish (front)', () => {
  // ---- TC-UT-F0003-001 ------------------------------------------------------
  test('t10_valid_move_triggers_reply', async () => {
    renderInto(STANDARD)
    initSelection('u-1')
    vi.spyOn(infraMove, 'movePiece').mockResolvedValue({
      status: 'valid', from: 'd2', to: 'd4', piece: 'P', capture: null, overview: STANDARD,
    })
    const applyValidSpy = vi.spyOn(applyValidMod, 'applyValidMove').mockImplementation(() => {})
    const playSpy = vi.spyOn(opponentMod, 'playOpponent').mockImplementation(async () => {})

    onSquareClick('d2')
    onSquareClick('d4')
    await flush()

    // F-1: the reply is triggered once, with the retained uuid.
    expect(playSpy).toHaveBeenCalledTimes(1)
    expect(playSpy).toHaveBeenCalledWith('u-1')
    // F-1: the trigger fires AFTER White's move is drawn.
    expect(applyValidSpy).toHaveBeenCalledTimes(1)
    expect(applyValidSpy.mock.invocationCallOrder[0]).toBeLessThan(playSpy.mock.invocationCallOrder[0])
  })

  // ---- TC-UT-F0003-002 ------------------------------------------------------
  test('t20_opponent_move_resolves_200', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ status: 200, ok: true, json: async () => MOVE_REPLY })
    vi.stubGlobal('fetch', fetchMock)

    const res = await opponentMove({ uuid: 'u-1' })

    // infra maps 200 to the full OpponentMoveResponse (incl. the nested overview).
    expect(res.from).toBe('c7')
    expect(res.to).toBe('c5')
    expect(res.piece).toBe('p')
    expect(res.capture).toBeNull()
    expect(res.status).toBe('move')
    expect(res.overview).toEqual(E4_AFTER_C5)
    // F-5: one POST to /opponent-move with a body deep-equal to { uuid }.
    expect(fetchMock).toHaveBeenCalledTimes(1)
    const [url, opts] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(String(url)).toContain('/opponent-move')
    expect(opts.method).toBe('POST')
    expect(JSON.parse(String(opts.body))).toEqual({ uuid: 'u-1' })
    // infra creates no DOM (role split, coding-rules §1).
    expect(document.body.childElementCount).toBe(0)
  })

  // ---- TC-UT-F0003-003 ------------------------------------------------------
  test('t30_opponent_move_throws_other_status', async () => {
    // 502 — the engine is down, a real error (F-5/F-6).
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 502, ok: false, json: async () => ({ error: 'engine unavailable' }) }))
    await expect(opponentMove({ uuid: 'u-1' })).rejects.toThrow()

    // 404 — a real error (F-5).
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 404, ok: false, json: async () => ({ error: 'unknown game' }) }))
    await expect(opponentMove({ uuid: 'u-1' })).rejects.toThrow()

    // 400 — a real error (F-5).
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 400, ok: false, json: async () => ({ error: 'invalid opponent-move request' }) }))
    await expect(opponentMove({ uuid: 'u-1' })).rejects.toThrow()

    // network down — a transport failure (F-5).
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network down')))
    await expect(opponentMove({ uuid: 'u-1' })).rejects.toThrow()
  })

  // ---- TC-UT-F0003-004 ------------------------------------------------------
  test('t40_play_opponent_applies_or_logs_failure', async () => {
    // Row 1 — resolves: applyOpponentMove is called with the reply (F-5).
    const oppSpy = vi.spyOn(infraOpp, 'opponentMove').mockResolvedValue(MOVE_REPLY)
    const applySpy = vi.spyOn(applyOppMod, 'applyOpponentMove').mockImplementation(() => {})

    await playOpponent('u-1')
    expect(applySpy).toHaveBeenCalledTimes(1)
    expect(applySpy).toHaveBeenCalledWith(MOVE_REPLY)

    // Row 2 — throws: applyOpponentMove not called, a log error appears, board unchanged (F-6).
    const root = renderWithLog(E4_BEFORE)
    const before = renderedPieces(root)
    applySpy.mockClear()
    oppSpy.mockRejectedValueOnce(new Error('engine down'))

    await playOpponent('u-1')
    expect(applySpy).not.toHaveBeenCalled() // F-6: no partial reply
    expect(logEntries().length).toBeGreaterThan(0) // F-6: an engine-error message is shown
    expect(renderedPieces(root)).toEqual(before) // F-6: the board is left as it was
  })

  // ---- TC-UT-F0003-005 ------------------------------------------------------
  test('t50_move_reply_redraws_borders_logs', () => {
    renderWithLog(E4_BEFORE)

    applyOpponentMove(MOVE_REPLY)

    // F-2: the reply is applied by redrawing from resp.overview.
    expect(pieceOn('c7')).toBe('') // source emptied
    expect(pieceOn('c5')).toBe('p') // target holds the moved black pawn
    // F-3: the departure square is red, the arrival square is blue.
    expect(hasClass('c7', 'last-move-from')).toBe(true)
    expect(hasClass('c5', 'last-move-to')).toBe(true)
    // exactly those two squares carry a reply border; no selected highlight added.
    expect(replyBorderCount()).toBe(2)
    expect(document.querySelectorAll('[data-square].selected')).toHaveLength(0)
    // F-4: the log gains one entry carrying the move and status.
    const entries = logEntries()
    expect(entries).toHaveLength(1)
    expect(entries[0]).toContain('c7')
    expect(entries[0]).toContain('c5')
    expect(entries[0]).toContain('move')
  })

  // ---- TC-UT-F0003-006 ------------------------------------------------------
  test('t60_smart_redraw_patches_changed_only', () => {
    const root = renderWithLog(CAP_BEFORE)
    // capture the exact piece nodes on the squares that do not change.
    const e1Node = root.querySelector('[data-square="e1"] [data-piece]')
    const e8Node = root.querySelector('[data-square="e8"] [data-piece]')

    applyOpponentMove(CAPTURE_REPLY)

    // 1. the black pawn lands on the target and overwrites the taken queen (F-2).
    expect(root.querySelector('[data-square="e6"] [data-piece]')).toBeNull() // e6 emptied
    expect(pieceOn('d5')).toBe('p') // d5 now holds the moved pawn
    // 2. unchanged squares keep the SAME DOM node — only changed squares patched (F-2).
    expect(root.querySelector('[data-square="e1"] [data-piece]')).toBe(e1Node)
    expect(root.querySelector('[data-square="e8"] [data-piece]')).toBe(e8Node)
    // 3. the grid is never rebuilt (F-2, and F0001 F-4 carried forward).
    expect(root.querySelectorAll('[data-square]')).toHaveLength(64)
  })

  // ---- TC-UT-F0003-007 ------------------------------------------------------
  test('t70_game_over_reply_shows_status', () => {
    const root = renderWithLog(CAP_BEFORE)
    const e1Node = root.querySelector('[data-square="e1"] [data-piece]')
    const d5Node = root.querySelector('[data-square="d5"] [data-piece]')

    applyOpponentMove(GAME_OVER_REPLY)

    // F-3: there is no move to border.
    expect(replyBorderCount()).toBe(0)
    // F-2: nothing is redrawn when the move is null and the board is unchanged.
    expect(root.querySelector('[data-square="e1"] [data-piece]')).toBe(e1Node)
    expect(root.querySelector('[data-square="d5"] [data-piece]')).toBe(d5Node)
    // F-4: the terminal status is shown.
    expect(logEntries().some((t) => t.includes('black-in-checkmate'))).toBe(true)
  })

  // ---- TC-UT-F0003-008 ------------------------------------------------------
  test('t80_new_source_clears_reply_borders', () => {
    renderInto(STANDARD)
    initSelection('u-1')
    const spy = vi.spyOn(infraMove, 'movePiece')
    // The last reply's borders: d7 departure (red), d5 arrival (blue).
    document.querySelector('[data-square="d7"]')!.classList.add('last-move-from')
    document.querySelector('[data-square="d5"]')!.classList.add('last-move-to')

    onSquareClick('d2') // a first click on a fresh white source

    // F-3: a new White move clears the black-reply borders (via F-9's reset).
    expect(hasClass('d7', 'last-move-from')).toBe(false)
    expect(hasClass('d5', 'last-move-to')).toBe(false)
    // F-9/F-1: only the current selection — the new source — is highlighted.
    expect(hasClass('d2', 'selected')).toBe(true)
    expect(document.querySelectorAll('[data-square].selected')).toHaveLength(1)
    // F-1/F-4: a first click sends no request (kept here for traceability).
    expect(spy).not.toHaveBeenCalled()
  })
})
