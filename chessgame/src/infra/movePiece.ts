// infra — the single network seam for moves (rule F-7). No game logic, no DOM.

import type { Overview } from '../domain/overview'
import { API_BASE } from './startGame'

/** The move to send: the retained game `uuid` and the two clicked squares. */
export interface MoveRequest {
  uuid: string
  from: string
  to: string
}

/**
 * The seam's resolved outcome — the HTTP status carries the verdict (rule F-7):
 * `200` → a `valid` applied move (with the authoritative board after it), `422`
 * → an `illegal` move carrying the closed-set `reason`.
 */
export type MoveResponse =
  | { status: 'valid'; from: string; to: string; piece: string; capture: string | null; overview: Overview }
  | { status: 'illegal'; reason: string }

/**
 * Call `POST /move-a-piece` with `{ uuid, from, to }` and map the response:
 * `200` → a `valid` outcome, `422` → an `illegal` `{ reason }`. Any other status
 * (`400`/`404`/network) is a real error and is **thrown** (rule F-7).
 */
export async function movePiece(req: MoveRequest): Promise<MoveResponse> {
  const resp = await fetch(`${API_BASE}/move-a-piece`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  })

  if (resp.status === 200) {
    const body = await resp.json()
    return {
      status: 'valid',
      from: body.from,
      to: body.to,
      piece: body.piece,
      capture: body.capture,
      overview: body.overview,
    }
  }
  if (resp.status === 422) {
    const body = await resp.json()
    return { status: 'illegal', reason: body.reason }
  }
  throw new Error(`move-a-piece failed: ${resp.status}`)
}
