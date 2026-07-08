// events — DOM writer: show an illegal-move message and clear the selection (F-6).

import { logError } from './logPanel'

/**
 * Handle an illegal move (rule F-6): write `message` (the `reason`) to the page's
 * `#message` live region, append it as a red entry in the side log panel, and
 * remove every `selected` highlight, cancelling the selection. The board itself
 * is left as it was.
 */
export function rejectMove(message: string): void {
  const msg = document.getElementById('message')
  if (msg) msg.textContent = message
  logError(message)
  document
    .querySelectorAll('[data-square]')
    .forEach((el) => el.classList.remove('selected', 'selected-source', 'selected-target'))
}
