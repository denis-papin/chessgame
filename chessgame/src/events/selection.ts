// events — the two-click selection state machine (rules F-1..F-6, F-8). Owns the
// DOM, the selection state, and orchestration; delegates classification to
// `domain`, the network to `infra`, and the DOM writes to the sibling routines.

import { isWhitePiece } from '../domain/pieces'
import * as infra from '../infra/movePiece'
import type { MoveResponse } from '../infra/movePiece'
import * as apply from './applyValidMove'
import * as reject from './rejectMove'
import * as opponent from './playOpponent'
import { logInfo } from './logPanel'

/** The active game `uuid`, retained from F0001's `GET /start-game` (rule F-8). */
let activeUuid = ''
/** The first-clicked source square, or `null` when nothing is selected. */
let square1: string | null = null

/**
 * Set the active game `uuid` and clear any selection (rule F-8). The seam
 * `onPageLoad` uses to retain the `uuid` for every move; a rematch / new game
 * calls it again with the fresh `uuid`.
 */
export function initSelection(uuid: string): void {
  activeUuid = uuid
  square1 = null
  clearHighlights()
}

/** The highlight classes a square may carry: `selected` (the state marker the
 * spec/tests pin) plus a role class that colours it — blue source / red target. */
const HIGHLIGHT_CLASSES = ['selected', 'selected-source', 'selected-target']

/** The black-reply borders left by the last opponent move (F0003 rule F-3). */
const REPLY_BORDER_CLASSES = ['last-move-from', 'last-move-to']

/** Remove every White-selection highlight and every black-reply border from every
 * square (rules F-9 and F0003 F-3: a new White move clears the reply borders). */
function clearHighlights(): void {
  document
    .querySelectorAll('[data-square]')
    .forEach((el) => el.classList.remove(...HIGHLIGHT_CLASSES, ...REPLY_BORDER_CLASSES))
}

/** Highlight `square` as the `source` (blue) or `target` (red) of the move. */
function highlight(square: string, role: 'source' | 'target'): void {
  document.querySelector(`[data-square="${square}"]`)?.classList.add('selected', `selected-${role}`)
}

function unhighlight(square: string): void {
  document.querySelector(`[data-square="${square}"]`)?.classList.remove(...HIGHLIGHT_CLASSES)
}

/**
 * Build the green success line from the API's valid-move JSON: `"P d2 → d4"`, or
 * `"P e4 × d5 (captured p)"` when the move takes a piece.
 */
function describeMove(res: Extract<MoveResponse, { status: 'valid' }>): string {
  const arrow = res.capture ? '×' : '→'
  const suffix = res.capture ? ` (captured ${res.capture})` : ''
  return `${res.piece} ${res.from} ${arrow} ${res.to}${suffix}`
}

/** Read the piece letter rendered on `square` (`""` when the square is empty). */
function pieceOn(square: string): string {
  const sq = document.querySelector(`[data-square="${square}"]`)
  return sq?.querySelector<HTMLElement>('[data-piece]')?.dataset.piece ?? ''
}

/**
 * The click state machine. First click on a white piece sets `square1` and
 * highlights it (rules F-1/F-2); re-clicking `square1` deselects (rule F-3); a
 * second, different click sets `square2`, highlights it, and **immediately**
 * fires the move (rule F-4). On the response, a `valid` outcome redraws from the
 * returned board (rule F-5) and an `illegal` one shows the reason and clears the
 * selection (rule F-6).
 */
export function onSquareClick(square: string): void {
  // First click — pick a white source (rules F-1/F-2).
  if (square1 === null) {
    if (!isWhitePiece(pieceOn(square))) return // empty or black → select nothing (F-2)
    clearHighlights() // F-9: reset any previous move's highlights before selecting
    square1 = square
    highlight(square, 'source') // F-1: source square highlighted (blue)
    return
  }

  // Re-click the held source — deselect (rule F-3).
  if (square === square1) {
    unhighlight(square)
    square1 = null
    return
  }

  // Second, different click — fire the move (rule F-4).
  const from = square1
  const to = square
  highlight(to, 'target') // F-4: target square highlighted (red)
  square1 = null // selection state resets; DOM highlights persist until apply/reject

  infra
    .movePiece({ uuid: activeUuid, from, to })
    .then((res) => {
      if (res.status === 'valid') {
        apply.applyValidMove(res.overview) // F-5: redraw from the returned board
        logInfo(describeMove(res)) // green: the move the API confirmed
        opponent.playOpponent(activeUuid) // F0003 F-1: trigger Black's reply
      } else {
        reject.rejectMove(res.reason) // F-6: show the reason, clear the selection
      }
    })
    .catch((err) => {
      // A real error (400/404/network) is not an illegal verdict (rule F-7):
      // clear the selection and leave the board untouched (no partial move).
      clearHighlights()
      console.error('move-a-piece error', err)
    })
}
