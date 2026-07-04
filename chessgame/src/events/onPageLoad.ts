// events — orchestrator wired to DOMContentLoaded. Touches all three layers
// (rule F-8): reads the page controls, calls infra, builds the model, renders.

import * as infra from '../infra/startGame'
import * as view from './renderBoard'
import { buildBoard } from '../domain/overview'
import type { Mode } from '../infra/startGame'

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
  const resp = await infra.startGame(readControls())
  const model = buildBoard(resp.overview)
  if (root) view.renderBoard(root, model)
}
