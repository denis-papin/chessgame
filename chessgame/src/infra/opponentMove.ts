// infra — the single network seam for the opponent reply (rule F-5). No DOM, no
// game logic. `POST /opponent-move` with `{ uuid }`, resolving `200` into the
// reply and throwing on any other status (400/404/502/network).

import type { Overview } from '../domain/overview'
import { API_BASE } from './startGame'

/** The reply request: just the retained game `uuid` (rule F-5). */
export interface OpponentMoveRequest {
  uuid: string
}

/**
 * The reply the server computed for Black. `from`/`to`/`piece`/`capture` are
 * `null` when the game was already over (Black had no move). `status` is one of
 * `move` / `white-in-check` / `white-in-checkmate` / `black-in-checkmate` /
 * `black-in-stalemate`.
 */
export interface OpponentMoveResponse {
  from: string | null
  to: string | null
  piece: string | null
  capture: string | null
  status: string
  overview: Overview
}

/**
 * Call `POST /opponent-move` with `{ uuid }`. A `200` resolves into the
 * `OpponentMoveResponse`; any other status (`400`/`404`/`502`/network) is a real
 * error and is **thrown** (rule F-5), so `events` can show the engine-failure
 * message (rule F-6).
 */
export async function opponentMove(req: OpponentMoveRequest): Promise<OpponentMoveResponse> {
  const resp = await fetch(`${API_BASE}/opponent-move`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  })

  if (resp.status === 200) {
    const body = await resp.json()
    return {
      from: body.from,
      to: body.to,
      piece: body.piece,
      capture: body.capture,
      status: body.status,
      overview: body.overview,
    }
  }
  throw new Error(`opponent-move failed: ${resp.status}`)
}
