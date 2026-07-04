// infra — the single network seam to fisher-server (rule F-8). No game logic.

import type { Overview } from '../domain/overview'

/** Back-end base URL — the one place the host is configured (coding-rules §1). */
export const API_BASE = 'http://localhost:7200'

export type Mode = 'standard' | 'random'

export interface StartGameResponse {
  uuid: string
  mode: string
  overview: Overview
}

export interface StartGameOpts {
  mode?: Mode
  /** Pieces per colour, random mode only (2..16). */
  pieces?: number
}

/**
 * Call `GET /start-game`, encoding `mode` and (for random) `pieces` into the
 * query string, and parse `{ uuid, mode, overview }`.
 */
export async function startGame(opts: StartGameOpts = {}): Promise<StartGameResponse> {
  const params = new URLSearchParams()
  if (opts.mode) params.set('mode', opts.mode)
  if (opts.mode === 'random' && opts.pieces != null) params.set('pieces', String(opts.pieces))

  const qs = params.toString()
  const url = `${API_BASE}/start-game${qs ? `?${qs}` : ''}`

  const resp = await fetch(url)
  if (!resp.ok) throw new Error(`start-game failed: ${resp.status}`)
  return (await resp.json()) as StartGameResponse
}
