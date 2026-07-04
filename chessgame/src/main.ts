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
  <main id="board" class="board"></main>
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
