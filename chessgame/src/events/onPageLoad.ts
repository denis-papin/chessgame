// events — orchestrator wired to DOMContentLoaded. Touches all three layers
// (rule F-8): reads the page controls, calls infra, builds the model, renders.

import * as infra from '../infra/startGame'
import * as view from './renderBoard'
import { buildBoard } from '../domain/overview'
import type { Mode } from '../infra/startGame'
import { initSelection, onSquareClick } from './selection'
import { logInfo } from './logPanel'

/** Read the current mode + piece-count from the page controls. */
function readControls(): { mode?: Mode; pieces?: number } {
  const modeEl = document.getElementById('mode') as HTMLSelectElement | null
  const piecesEl = document.getElementById('pieces') as HTMLInputElement | null
  const mode = (modeEl?.value as Mode) || 'standard'
  if (mode === 'random') {
    return { mode, pieces: piecesEl ? Number(piecesEl.value) : undefined }
  }
  return { mode }
}

/**
 * Start-game flow: `startGame` -> `buildBoard` -> `renderBoard`. If the call
 * fails, the rejection surfaces and the board is left untouched (no partial
 * board); this routine does not swallow the error.
 */
export async function onPageLoad(): Promise<void> {
  const root = document.getElementById('board')
  const controls = readControls()
  const resp = await infra.startGame(controls)
  const model = buildBoard(resp.overview)
  if (root) {
    view.renderBoard(root, model)
    initSelection(resp.uuid) // F-8: retain the game uuid for every move
    wireClicks(root) // F-8: click-to-move on the board
    logInfo(`New game started (${controls.mode})`) // green: front-side event
  }
}

/**
 * Wire click-to-move on `#board` once (rule F-8). Delegates to the selection
 * machine, resolving the clicked `data-square` from the event target. Guarded so
 * a rematch (which re-renders into the same root) does not stack listeners.
 */
function wireClicks(root: HTMLElement): void {
  if (root.dataset.clickWired) return
  root.dataset.clickWired = 'true'
  root.addEventListener('click', (e) => {
    const square = (e.target as HTMLElement).closest<HTMLElement>('[data-square]')?.dataset.square
    if (square) onSquareClick(square)
  })
}
