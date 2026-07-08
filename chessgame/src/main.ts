import './style.css'
import { onPageLoad } from './events/onPageLoad'

const app = document.querySelector<HTMLDivElement>('#app')!

app.innerHTML = `
  <header class="controls">
    <label>Mode
      <select id="mode">
        <option value="standard">standard</option>
        <option value="random">random</option>
      </select>
    </label>
    <label>Pieces
      <input id="pieces" type="number" min="2" max="16" value="8" />
    </label>
    <button id="rematch" type="button">Rematch</button>
  </header>
  <div class="layout">
    <main id="board" class="board"></main>
    <aside id="log-panel" class="log-panel" aria-label="Log">
      <h2 class="log-panel__title">Log</h2>
      <ul id="log-list" class="log-list"></ul>
      <p id="log-empty" class="log-empty">No messages yet.</p>
    </aside>
  </div>
  <!-- Live region kept for assistive tech; the panel is the visible display. -->
  <p id="message" class="message visually-hidden" role="status"></p>
`

// A new game is fetched on load and whenever the mode, the piece count, or the
// rematch button changes (Inputs section of F0001).
const refresh = (): void => {
  onPageLoad().catch((err) => console.error('start-game failed', err))
}

document.getElementById('mode')!.addEventListener('change', refresh)
document.getElementById('pieces')!.addEventListener('change', refresh)
document.getElementById('rematch')!.addEventListener('click', refresh)

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', refresh)
} else {
  refresh()
}
