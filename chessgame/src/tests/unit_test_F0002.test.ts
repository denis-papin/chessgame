// Front unit tests for feature F0002 (UT-F0002) — move a piece by two clicks,
// redraw from the returned board. Vitest + jsdom. No real back end (fetch /
// infra mocked) and no real browser. The board model is the project's own
// `Overview`, not FEN. The F0001 render helpers are reused, not re-proven.

import { afterEach, describe, expect, test, vi } from 'vitest'

import { buildBoard } from '../domain/overview'
import type { Overview } from '../domain/overview'
import { renderBoard } from '../events/renderBoard'

import { isWhitePiece } from '../domain/pieces'
import { movePiece } from '../infra/movePiece'
import * as infraMove from '../infra/movePiece'
import * as applyMod from '../events/applyValidMove'
import * as rejectMod from '../events/rejectMove'
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

// STANDARD after d2 -> d4 (d2 emptied, d4 filled).
const AFTER_D2D4: Overview = {
  board: [
    ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
    ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', 'P', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['P', 'P', 'P', '', 'P', 'P', 'P', 'P'],
    ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
  ],
  white: 'both',
  black: 'both',
}

// Capture position: White P e4, Black p d5, kings e1/e8.
const CAPTURE_BEFORE: Overview = {
  board: [
    ['', '', '', '', 'k', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', 'p', '', '', '', ''],
    ['', '', '', '', 'P', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', 'K', '', '', ''],
  ],
  white: 'both',
  black: 'both',
}

// CAPTURE_BEFORE after e4 -> d5 (the pawn takes and lands on d5, e4 emptied).
const CAPTURE_AFTER: Overview = {
  board: [
    ['', '', '', '', 'k', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', '', 'P', '', '', '', ''],
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

const isSelected = (square: string) =>
  document.querySelector(`[data-square="${square}"]`)!.classList.contains('selected')

const anySelected = () => document.querySelectorAll('[data-square].selected').length > 0

const pieceOn = (square: string) =>
  document.querySelector(`[data-square="${square}"] [data-piece]`)?.getAttribute('data-piece') ?? ''

/** Flush pending microtasks so a settled `movePiece` promise resolves. */
const flush = () => new Promise<void>((r) => setTimeout(r, 0))

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  document.body.innerHTML = ''
  initSelection('') // reset the module's selection state between tests
})

describe('UT-F0002 — move a piece (front)', () => {
  // ---- TC-UT-F0002-001 ------------------------------------------------------
  test('t10_is_white_piece', () => {
    expect(isWhitePiece('P')).toBe(true) // uppercase white piece (F-1)
    expect(isWhitePiece('K')).toBe(true)
    expect(isWhitePiece('p')).toBe(false) // lowercase black piece (F-2)
    expect(isWhitePiece('k')).toBe(false)
    expect(isWhitePiece('')).toBe(false) // empty square — nothing to move (F-2)
  })

  // ---- TC-UT-F0002-002 ------------------------------------------------------
  test('t20_first_click_selects_white', () => {
    renderInto(STANDARD)
    initSelection('u-1')
    const spy = vi.spyOn(infraMove, 'movePiece')

    onSquareClick('d2') // d2 holds a white pawn

    expect(isSelected('d2')).toBe(true) // F-1: the white source is highlighted
    expect(document.querySelectorAll('[data-square].selected')).toHaveLength(1) // only square1 held
    expect(spy).not.toHaveBeenCalled() // F-1/F-4: the first click sends nothing
  })

  // ---- TC-UT-F0002-003 ------------------------------------------------------
  test('t30_first_click_empty_or_black_refused', () => {
    renderInto(STANDARD)
    const spy = vi.spyOn(infraMove, 'movePiece')

    initSelection('u-1')
    onSquareClick('e4') // empty in the opening
    expect(anySelected()).toBe(false) // F-2: an empty source selects nothing
    expect(spy).not.toHaveBeenCalled()

    initSelection('u-1')
    onSquareClick('d7') // black pawn — White only
    expect(anySelected()).toBe(false) // F-2: a black source selects nothing
    expect(spy).not.toHaveBeenCalled()
  })

  // ---- TC-UT-F0002-004 ------------------------------------------------------
  test('t40_reclick_deselects', () => {
    renderInto(STANDARD)
    initSelection('u-1')
    const spy = vi.spyOn(infraMove, 'movePiece')
    onSquareClick('d2') // first click selects d2

    onSquareClick('d2') // re-click the same square

    expect(isSelected('d2')).toBe(false) // F-3: re-click removes the highlight
    expect(anySelected()).toBe(false) // back to the no-selection state
    expect(spy).not.toHaveBeenCalled() // F-3: a deselect sends no move
  })

  // ---- TC-UT-F0002-005 ------------------------------------------------------
  test('t50_second_click_sends_move', () => {
    renderInto(STANDARD)
    initSelection('u-1')
    const spy = vi
      .spyOn(infraMove, 'movePiece')
      .mockResolvedValue({ status: 'valid', from: 'd2', to: 'd4', piece: 'P', capture: null, overview: AFTER_D2D4 })
    onSquareClick('d2') // first click selects d2

    onSquareClick('d4') // a different square

    // F-4: the second click sends the retained uuid + the two squares, once.
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy).toHaveBeenCalledWith({ uuid: 'u-1', from: 'd2', to: 'd4' })
    // both squares are highlighted at the moment the request goes out (F-4).
    expect(isSelected('d2')).toBe(true)
    expect(isSelected('d4')).toBe(true)
  })

  // ---- TC-UT-F0002-006 ------------------------------------------------------
  test('t60_valid_applies_and_keeps_highlights', async () => {
    renderInto(STANDARD)
    initSelection('u-1')
    vi.spyOn(infraMove, 'movePiece').mockResolvedValue({
      status: 'valid',
      from: 'd2',
      to: 'd4',
      piece: 'P',
      capture: null,
      overview: AFTER_D2D4,
    })
    const applySpy = vi.spyOn(applyMod, 'applyValidMove').mockImplementation(() => {})
    const rejectSpy = vi.spyOn(rejectMod, 'rejectMove').mockImplementation(() => {})

    onSquareClick('d2')
    onSquareClick('d4')
    await flush()

    // F-5: a valid move is applied by redrawing from the returned board.
    expect(applySpy).toHaveBeenCalledTimes(1)
    expect(applySpy).toHaveBeenCalledWith(AFTER_D2D4)
    expect(rejectSpy).not.toHaveBeenCalled() // valid/illegal branches are exclusive
    // the redraw leaves both squares highlighted (F-5).
    expect(isSelected('d2')).toBe(true)
    expect(isSelected('d4')).toBe(true)
  })

  // ---- TC-UT-F0002-007 ------------------------------------------------------
  test('t70_illegal_shows_reason_and_clears', async () => {
    const root = renderInto(STANDARD)
    root.insertAdjacentHTML('afterend', '<p id="message"></p>')
    initSelection('u-1')
    vi.spyOn(infraMove, 'movePiece').mockResolvedValue({ status: 'illegal', reason: 'path blocked' })
    const before = renderedPieces(root)

    onSquareClick('d1') // white queen
    onSquareClick('d3') // fires the (illegal) move
    await flush()

    // F-6: the reason is shown on the page.
    expect(document.getElementById('message')!.textContent).toContain('path blocked')
    // F-6: the selection and both highlights are cleared.
    expect(anySelected()).toBe(false)
    // F-6: an illegal move applies nothing on the front — the board is unchanged.
    expect(renderedPieces(root)).toEqual(before)
  })

  // ---- TC-UT-F0002-008 ------------------------------------------------------
  test('t80_move_piece_valid_and_seam', async () => {
    const payload = { from: 'd2', to: 'd4', piece: 'P', capture: null, overview: AFTER_D2D4 }
    const fetchMock = vi.fn().mockResolvedValue({ status: 200, ok: true, json: async () => payload })
    vi.stubGlobal('fetch', fetchMock)

    const res = await movePiece({ uuid: 'u-1', from: 'd2', to: 'd4' })

    // infra maps 200 to a valid outcome carrying the Output shape (incl. overview).
    expect(res.status).toBe('valid')
    if (res.status === 'valid') {
      expect(res.from).toBe('d2')
      expect(res.to).toBe('d4')
      expect(res.piece).toBe('P')
      expect(res.capture).toBeNull()
      expect(res.overview).toEqual(AFTER_D2D4)
    }
    // F-7: one POST to /move-a-piece with a body deep-equal to the request.
    expect(fetchMock).toHaveBeenCalledTimes(1)
    const [url, opts] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(String(url)).toContain('/move-a-piece')
    expect(opts.method).toBe('POST')
    expect(JSON.parse(String(opts.body))).toEqual({ uuid: 'u-1', from: 'd2', to: 'd4' })
    // infra creates no DOM (role split, coding-rules §1).
    expect(document.body.childElementCount).toBe(0)
  })

  // ---- TC-UT-F0002-009 ------------------------------------------------------
  test('t90_move_piece_illegal_422', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ status: 422, ok: false, json: async () => ({ reason: 'king in check' }) })
    vi.stubGlobal('fetch', fetchMock)

    const res = await movePiece({ uuid: 'u-1', from: 'e2', to: 'd3' })

    // F-7: a 422 is a normal illegal verdict, resolved rather than thrown.
    expect(res.status).toBe('illegal')
    if (res.status === 'illegal') expect(res.reason).toBe('king in check')
  })

  // ---- TC-UT-F0002-010 ------------------------------------------------------
  test('t100_move_piece_throws_other_status', async () => {
    // 404 — a real error, not a verdict (F-7).
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 404, ok: false, json: async () => ({ error: 'unknown game' }) }))
    await expect(movePiece({ uuid: 'u-1', from: 'd2', to: 'd4' })).rejects.toThrow()

    // 400 — a real error, not a verdict (F-7).
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 400, ok: false, json: async () => ({ error: 'invalid move request' }) }))
    await expect(movePiece({ uuid: 'u-1', from: 'd2', to: 'd4' })).rejects.toThrow()

    // network down — a transport failure is a real error (F-7).
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network down')))
    await expect(movePiece({ uuid: 'u-1', from: 'd2', to: 'd4' })).rejects.toThrow()
  })

  // ---- TC-UT-F0002-011 ------------------------------------------------------
  test('t110_smart_redraw_patches_changed_only', () => {
    const root = renderInto(CAPTURE_BEFORE)
    // capture the exact piece nodes on the squares that do not change.
    const e1Node = root.querySelector('[data-square="e1"] [data-piece]')
    const e8Node = root.querySelector('[data-square="e8"] [data-piece]')

    applyMod.applyValidMove(CAPTURE_AFTER)

    // 1. the moved pawn lands on the target and overwrites the taken piece (F-5).
    expect(root.querySelector('[data-square="e4"] [data-piece]')).toBeNull() // e4 emptied
    expect(pieceOn('d5')).toBe('P') // d5 now holds the moved pawn
    // 2. unchanged squares keep the SAME DOM node — only changed squares patched (F-5).
    expect(root.querySelector('[data-square="e1"] [data-piece]')).toBe(e1Node)
    expect(root.querySelector('[data-square="e8"] [data-piece]')).toBe(e8Node)
    // 3. the grid is never rebuilt (F-5, and F0001 F-4 carried forward).
    expect(root.querySelectorAll('[data-square]')).toHaveLength(64)
  })

  // ---- TC-UT-F0002-012 ------------------------------------------------------
  test('t120_new_selection_resets_old_highlights', () => {
    renderInto(STANDARD)
    initSelection('u-1')
    const spy = vi.spyOn(infraMove, 'movePiece')
    // Simulate a previous move's leftover highlights (F-5): d2 source, d4 target.
    document.querySelector('[data-square="d2"]')!.classList.add('selected', 'selected-source')
    document.querySelector('[data-square="d4"]')!.classList.add('selected', 'selected-target')

    onSquareClick('e2') // a first click on a fresh white source

    // F-9: the previous move's highlights are reset before the new source is coloured.
    expect(isSelected('d2')).toBe(false)
    expect(isSelected('d4')).toBe(false)
    // F-9/F-1: only the current selection — the new source — is highlighted.
    expect(isSelected('e2')).toBe(true)
    expect(document.querySelectorAll('[data-square].selected')).toHaveLength(1)
    // F-1/F-4: a first click sends no request (kept here for traceability).
    expect(spy).not.toHaveBeenCalled()
  })
})

/** Occupied squares as `{ square, piece }`, sorted — for the unchanged-board assertion. */
function renderedPieces(root: HTMLElement) {
  return Array.from(root.querySelectorAll<HTMLElement>('[data-piece]'))
    .map((el) => ({ square: (el.parentElement as HTMLElement).dataset.square, piece: el.dataset.piece }))
    .sort((a, b) => (a.square ?? '').localeCompare(b.square ?? ''))
}
