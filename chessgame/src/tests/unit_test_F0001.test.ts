// Front unit tests for feature F0001 (UT-F0001).
// Vitest + jsdom. No real back end (fetch mocked) and no real browser.

import { afterEach, describe, expect, test, vi } from 'vitest'

import { buildBoard, cells, squareColor } from '../domain/overview'
import type { BoardModel, Overview } from '../domain/overview'
import { startGame } from '../infra/startGame'
import { renderBoard } from '../events/renderBoard'
import * as infra from '../infra/startGame'
import * as view from '../events/renderBoard'
import { onPageLoad } from '../events/onPageLoad'

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

const SPARSE: Overview = {
  board: [
    ['', '', '', '', 'k', '', '', ''], // k e8
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', '', '', '', ''],
    ['', '', 'N', '', '', '', '', ''], // N c5
    ['', '', '', '', '', 'q', '', ''], // q f4
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', 'P', '', '', ''], // P e2
    ['', '', '', '', 'K', '', '', ''], // K e1
  ],
  white: 'none',
  black: 'none',
}

// pieces=4 (4 white + 4 black, one king each) — matches the F0001 example.
const RANDOM4: Overview = {
  board: [
    ['', '', '', '', 'k', '', '', ''], // k e8
    ['', '', '', 'p', '', '', '', ''], // p d7
    ['', 'n', '', '', '', '', '', ''], // n b6
    ['', '', 'N', '', '', '', '', ''], // N c5
    ['', '', '', '', '', 'q', '', ''], // q f4
    ['', '', '', '', '', '', '', ''],
    ['', '', '', '', 'P', '', '', ''], // P e2
    ['', '', '', 'Q', 'K', '', '', ''], // Q d1, K e1
  ],
  white: 'none',
  black: 'none',
}

// ---- helpers ----------------------------------------------------------------

const bySquare = (a: { square?: string }, b: { square?: string }) =>
  (a.square ?? '').localeCompare(b.square ?? '')

function renderedPieces(root: HTMLElement) {
  return Array.from(root.querySelectorAll<HTMLElement>('[data-piece]'))
    .map((el) => ({ square: (el.parentElement as HTMLElement).dataset.square, piece: el.dataset.piece }))
    .sort(bySquare)
}

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  document.body.innerHTML = ''
})

// ---- TC-UT-F0001-001 --------------------------------------------------------

describe('UT-F0001 — start a game (front render)', () => {
  test('t10_cells_standard_32_squares', () => {
    const out = cells(STANDARD)
    expect(out).toHaveLength(32) // F-5
    const at = (sq: string) => out.find((c) => c.square === sq)?.piece
    expect(at('a8')).toBe('r') // F-3
    expect(at('e8')).toBe('k')
    expect(at('e1')).toBe('K')
    expect(at('a1')).toBe('R')
    expect(at('h2')).toBe('P')
    // ranks 3-6 contribute nothing
    expect(out.some((c) => ['3', '4', '5', '6'].includes(c.square[1]))).toBe(false)
  })

  // ---- TC-UT-F0001-002 ------------------------------------------------------
  test('t20_cells_sparse_overview', () => {
    const out = cells(SPARSE)
    const expected = [
      { square: 'e8', piece: 'k' },
      { square: 'c5', piece: 'N' },
      { square: 'f4', piece: 'q' },
      { square: 'e2', piece: 'P' },
      { square: 'e1', piece: 'K' },
    ]
    expect(out).toHaveLength(5) // F-3
    expect([...out].sort(bySquare)).toEqual([...expected].sort(bySquare)) // F-5
  })

  // ---- TC-UT-F0001-003 ------------------------------------------------------
  test('t30_square_colour_parity', () => {
    expect(squareColor('a1')).toBe('dark')
    expect(squareColor('h1')).toBe('light')
    expect(squareColor('a8')).toBe('light')
    expect(squareColor('h8')).toBe('dark')
    expect(squareColor('e4')).toBe('light')
  })

  // ---- TC-UT-F0001-004 ------------------------------------------------------
  test('t40_board_model_64_cells', () => {
    const model = buildBoard(STANDARD)
    expect(model).toHaveLength(64) // F-1
    const coords = model.map((c) => c.square)
    expect(new Set(coords).size).toBe(64)
    for (const f of 'abcdefgh') for (const r of '12345678') expect(coords).toContain(`${f}${r}`)
    for (const c of model) expect(c.color).toBe(squareColor(c.square)) // F-2
    // visual order: rank 8 first, rank 1 last, a1 at the bottom-left
    expect(model[0].square).toBe('a8')
    expect(model[63].square).toBe('h1')
    expect(model[56].square).toBe('a1')
  })

  // ---- TC-UT-F0001-005 ------------------------------------------------------
  test('t50_render_grid_then_pieces', () => {
    document.body.innerHTML = '<div id="board"></div>'
    const root = document.getElementById('board') as HTMLElement
    renderBoard(root, buildBoard(STANDARD))

    expect(root.querySelectorAll('[data-square]')).toHaveLength(64) // F-1
    const pieces = root.querySelectorAll<HTMLElement>('[data-piece]')
    pieces.forEach((p) =>
      expect((p.parentElement as HTMLElement).hasAttribute('data-square')).toBe(true),
    ) // F-4
    expect(pieces).toHaveLength(32) // F-5
    expect(renderedPieces(root)).toEqual(
      cells(STANDARD).map((c) => ({ square: c.square, piece: c.piece })).sort(bySquare),
    )
    expect(root.querySelector('[data-square="a1"]')!.classList.contains('dark')).toBe(true) // F-2
    expect(root.querySelector('[data-square="h1"]')!.classList.contains('light')).toBe(true)
  })

  // ---- TC-UT-F0001-006 ------------------------------------------------------
  test('t60_edge_labels', () => {
    document.body.innerHTML = '<div id="board"></div>'
    const root = document.getElementById('board') as HTMLElement
    renderBoard(root, buildBoard(STANDARD))

    const files = Array.from(root.querySelectorAll('.file-label')).map((e) => e.textContent).join(' ')
    expect(files).toBe('a b c d e f g h') // F-6
    const ranks = Array.from(root.querySelectorAll('.rank-label')).map((e) => e.textContent).join(' ')
    expect(ranks).toBe('8 7 6 5 4 3 2 1')
  })

  // ---- TC-UT-F0001-007 ------------------------------------------------------
  test('t70_infra_start_game_parse_and_seam', async () => {
    const payload = { uuid: 'u-1', mode: 'standard', overview: STANDARD }
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => payload })
    vi.stubGlobal('fetch', fetchMock)

    const r1 = await startGame()
    expect(r1).toEqual(payload) // parses the Output shape incl. nested Overview
    expect(fetchMock).toHaveBeenCalledTimes(1) // F-8
    expect(String(fetchMock.mock.calls[0][0])).toContain('/start-game')
    expect(String(fetchMock.mock.calls[0][0])).not.toContain('?')

    fetchMock.mockClear()
    await startGame({ mode: 'random', pieces: 4 })
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(String(fetchMock.mock.calls[0][0])).toContain('/start-game?mode=random&pieces=4')

    expect(document.body.childElementCount).toBe(0) // infra creates no DOM
  })

  // ---- TC-UT-F0001-008 ------------------------------------------------------
  test('t80_on_page_load_orchestration', async () => {
    document.body.innerHTML =
      '<div id="board"></div>' +
      '<select id="mode"><option value="standard">standard</option><option value="random">random</option></select>' +
      '<input id="pieces" type="number" value="6" />'
    ;(document.getElementById('mode') as HTMLSelectElement).value = 'random'

    const startSpy = vi
      .spyOn(infra, 'startGame')
      .mockResolvedValue({ uuid: 'u-1', mode: 'standard', overview: STANDARD })
    const renderSpy = vi.spyOn(view, 'renderBoard').mockImplementation(() => {})

    await onPageLoad()

    // 1. API seam driven once, with the page's mode + pieces (B-3)
    expect(startSpy).toHaveBeenCalledTimes(1)
    expect(startSpy).toHaveBeenCalledWith({ mode: 'random', pieces: 6 })

    // 2. renderBoard called once with #board root and the model from the parsed Overview
    expect(renderSpy).toHaveBeenCalledTimes(1)
    const [rootArg, modelArg] = renderSpy.mock.calls[0] as [HTMLElement, BoardModel]
    expect(rootArg).toBe(document.getElementById('board'))
    const occupied = modelArg
      .filter((c) => c.piece)
      .map((c) => ({ square: c.square, piece: c.piece as string }))
    expect([...occupied].sort(bySquare)).toEqual(cells(STANDARD).sort(bySquare))

    // 3. on rejection, renderBoard is not called and the error surfaces
    startSpy.mockReset()
    startSpy.mockRejectedValue(new Error('back end down'))
    renderSpy.mockClear()
    await expect(onPageLoad()).rejects.toThrow('back end down')
    expect(renderSpy).not.toHaveBeenCalled()
  })

  // ---- TC-UT-F0001-009 ------------------------------------------------------
  test('t90_render_rerender_reuses_grid', () => {
    document.body.innerHTML = '<div id="board"></div>'
    const root = document.getElementById('board') as HTMLElement

    renderBoard(root, buildBoard(STANDARD))
    expect(root.querySelectorAll('[data-piece]')).toHaveLength(32)

    renderBoard(root, buildBoard(RANDOM4))

    expect(root.querySelectorAll('[data-square]')).toHaveLength(64) // F-4 grid reused, not duplicated
    const pieces = root.querySelectorAll<HTMLElement>('[data-piece]')
    expect(pieces).toHaveLength(8) // 4 white + 4 black
    expect(renderedPieces(root)).toEqual(
      cells(RANDOM4).map((c) => ({ square: c.square, piece: c.piece })).sort(bySquare),
    )
    // previous position fully cleared
    expect(root.querySelector('[data-square="a8"]')!.querySelector('[data-piece]')).toBeNull()
    pieces.forEach((p) =>
      expect((p.parentElement as HTMLElement).hasAttribute('data-square')).toBe(true),
    )
  })
})
