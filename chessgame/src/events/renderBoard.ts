// events — DOM writer for the board (rules F-1, F-4, F-5, F-6). Owns the DOM.

import type { BoardModel } from '../domain/overview'

const FILES = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h']
const RANKS = ['8', '7', '6', '5', '4', '3', '2', '1']

/** Lichess cburnett SVG for a FEN piece letter (e.g. "N" -> wN, "k" -> bK). */
function pieceSvgUrl(letter: string): string {
  const color = letter === letter.toUpperCase() ? 'w' : 'b'
  const type = letter.toUpperCase()
  return `https://lichess1.org/assets/piece/cburnett/${color}${type}.svg`
}

/** Build the 64-square grid once from the model (coords + colour). */
function buildGrid(root: HTMLElement, model: BoardModel): HTMLElement {
  root.replaceChildren()

  const grid = document.createElement('div')
  grid.className = 'board-grid'
  for (const cell of model) {
    const sq = document.createElement('div')
    sq.className = `square ${cell.color}`
    sq.dataset.square = cell.square
    grid.appendChild(sq)
  }
  root.appendChild(grid)

  root.appendChild(buildLabels('files', 'file-label', FILES))
  root.appendChild(buildLabels('ranks', 'rank-label', RANKS))
  return grid
}

function buildLabels(wrapClass: string, itemClass: string, values: string[]): HTMLElement {
  const wrap = document.createElement('div')
  wrap.className = wrapClass
  for (const v of values) {
    const el = document.createElement('span')
    el.className = itemClass
    el.textContent = v
    wrap.appendChild(el)
  }
  return wrap
}

/**
 * Render `model` into `root`. The 64-square grid is built on the first call and
 * **reused** on every later call (rematch / mode / pieces change, rule F-4):
 * only the pieces are refreshed, never the grid.
 */
export function renderBoard(root: HTMLElement, model: BoardModel): void {
  const grid =
    (root.querySelector('.board-grid') as HTMLElement | null) ?? buildGrid(root, model)

  // Clear the previous position, then place the new pieces onto existing squares.
  grid.querySelectorAll('[data-piece]').forEach((el) => el.remove())

  for (const cell of model) {
    if (!cell.piece) continue
    const sq = grid.querySelector(`[data-square="${cell.square}"]`)
    if (!sq) continue
    const img = document.createElement('img')
    img.className = 'piece'
    img.dataset.piece = cell.piece
    img.src = pieceSvgUrl(cell.piece)
    img.alt = cell.piece
    sq.appendChild(img)
  }
}
