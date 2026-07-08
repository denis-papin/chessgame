// domain — pure piece helpers for feature F0002. No DOM, no network.

/**
 * `true` when `letter` is a non-empty uppercase piece letter (`P N B R Q K`),
 * i.e. a white piece. `false` for a lowercase (black) letter or `""` (rule F-1).
 * Backs the `square1` front check that only a white piece may be selected as a
 * source; an empty or black square is refused before any request (rule F-2).
 */
export function isWhitePiece(letter: string): boolean {
  return /^[PNBRQK]$/.test(letter)
}
