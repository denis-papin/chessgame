// events — DOM writer: the smart redraw of a valid move (rule F-5). Owns the DOM.

import { buildBoard } from '../domain/overview'
import type { Overview } from '../domain/overview'

/** Lichess cburnett SVG for a FEN piece letter (mirrors renderBoard). */
function pieceSvgUrl(letter: string): string {
  const color = letter === letter.toUpperCase() ? 'w' : 'b'
  const type = letter.toUpperCase()
  return `https://lichess1.org/assets/piece/cburnett/${color}${type}.svg`
}

/**
 * Apply a valid move by **redrawing from the returned board** (rule F-5), not by
 * hand-moving pieces. Rebuilds the pure model with `buildBoard(overview)` and
 * patches **only** the squares whose piece differs from what is currently
 * rendered: an unchanged square's DOM node is left in place. For a normal move
 * exactly two squares change (source emptied, target filled); a capture changes
 * the same two (the taken piece sat on the target and is overwritten), so no
 * `capture` special-case is needed. The 64 grid cells are never rebuilt and the
 * `selected` highlights are left untouched.
 */
export function applyValidMove(overview: Overview): void {
  for (const cell of buildBoard(overview)) {
    const sq = document.querySelector(`[data-square="${cell.square}"]`)
    if (!sq) continue

    const current = sq.querySelector<HTMLElement>('[data-piece]')
    const rendered = current?.dataset.piece ?? ''
    const wanted = cell.piece ?? ''
    if (rendered === wanted) continue // unchanged → leave this square's DOM in place

    if (current) current.remove()
    if (wanted) {
      const img = document.createElement('img')
      img.className = 'piece'
      img.dataset.piece = wanted
      img.src = pieceSvgUrl(wanted)
      img.alt = wanted
      sq.appendChild(img)
    }
  }
}
