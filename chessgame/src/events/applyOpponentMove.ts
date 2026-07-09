// events — DOM writer for the opponent reply (rules F-2, F-3, F-4). Owns the DOM.
// Smart-redraws from the returned board, borders Black's move (departure red,
// arrival blue), and logs the move + status.

import { buildBoard } from '../domain/overview'
import type { OpponentMoveResponse } from '../infra/opponentMove'
import { logInfo } from './logPanel'

/** The black-reply border classes: departure (red) and arrival (blue). */
const REPLY_BORDER_CLASSES = ['last-move-from', 'last-move-to']

/** Lichess cburnett SVG for a FEN piece letter (mirrors renderBoard). */
function pieceSvgUrl(letter: string): string {
  const color = letter === letter.toUpperCase() ? 'w' : 'b'
  const type = letter.toUpperCase()
  return `https://lichess1.org/assets/piece/cburnett/${color}${type}.svg`
}

/** Remove the black-reply borders from every square. */
function clearReplyBorders(): void {
  document.querySelectorAll('[data-square]').forEach((el) => el.classList.remove(...REPLY_BORDER_CLASSES))
}

/**
 * Smart redraw from `overview` (rule F-2): patch **only** the squares whose piece
 * differs from what is rendered. A game-over reply carries the unchanged board,
 * so the diff is empty and no DOM node is touched. Mirrors `applyValidMove`.
 */
function smartRedraw(overview: OpponentMoveResponse['overview']): void {
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

/** The log line: `"p c7 → c5 — move"`, `"p e6 × d5 (captured Q) — white-in-check"`,
 * or `"(none) — black-in-checkmate"` when the game was already over. */
function describeReply(resp: OpponentMoveResponse): string {
  if (!resp.from || !resp.to) return `(none) — ${resp.status}`
  const arrow = resp.capture ? '×' : '→'
  const suffix = resp.capture ? ` (captured ${resp.capture})` : ''
  return `${resp.piece} ${resp.from} ${arrow} ${resp.to}${suffix} — ${resp.status}`
}

/**
 * Reflect the opponent reply: redraw from the returned board (rule F-2), border
 * Black's departure square red and arrival square blue (rule F-3), and append a
 * log entry carrying the move and status (rule F-4). On a game-over reply (a
 * `null` move) nothing is redrawn or bordered — only the status is shown.
 */
export function applyOpponentMove(resp: OpponentMoveResponse): void {
  // F-2 — redraw from the authoritative board (a game-over reply is a no-op diff).
  smartRedraw(resp.overview)

  // F-3 — border Black's move: clear the previous reply's borders, then mark this one.
  clearReplyBorders()
  if (resp.from && resp.to) {
    document.querySelector(`[data-square="${resp.from}"]`)?.classList.add('last-move-from')
    document.querySelector(`[data-square="${resp.to}"]`)?.classList.add('last-move-to')
  }

  // F-4 — log the move and status.
  logInfo(describeReply(resp))
}
