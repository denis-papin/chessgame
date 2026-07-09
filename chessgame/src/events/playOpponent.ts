// events — orchestrates the opponent reply (rules F-5, F-6). Fired right after a
// valid White move is applied (rule F-1). Calls the network seam, then reflects
// the reply; on an engine failure it logs a message and leaves the board as it is.

import * as infra from '../infra/opponentMove'
import * as apply from './applyOpponentMove'
import { logError } from './logPanel'

/**
 * Ask Stockfish for Black's reply and reflect it (rule F-5). On success the
 * reply is handed to `applyOpponentMove` (redraw + borders + log). If the seam
 * throws — a `502` engine failure or a network error — the front shows an
 * engine-error message and leaves the board and highlights untouched (rule F-6):
 * no partial reply.
 */
export async function playOpponent(uuid: string): Promise<void> {
  try {
    const resp = await infra.opponentMove({ uuid })
    apply.applyOpponentMove(resp)
  } catch (err) {
    logError('Stockfish is unavailable — please try again.')
    console.error('opponent-move error', err)
  }
}
