// events — DOM writer for the side log panel. Appends timestamped entries,
// newest first: red for errors, green for informational/success messages.

type LogLevel = 'error' | 'info'

/** Append a red error entry (e.g. an illegal-move reason). */
export function logError(message: string): void {
  appendEntry(message, 'error')
}

/** Append a green info entry (e.g. a successful move reported by the API). */
export function logInfo(message: string): void {
  appendEntry(message, 'info')
}

/**
 * Append `message` as a coloured entry (newest first) to the side panel's
 * `#log-list`. A no-op when the panel is absent (e.g. the jsdom test harness),
 * so callers keep working outside the full page. The one-time empty-state
 * placeholder is removed on the first entry.
 */
function appendEntry(message: string, level: LogLevel): void {
  const list = document.getElementById('log-list')
  if (!list) return

  document.getElementById('log-empty')?.remove()

  const entry = document.createElement('li')
  entry.className = `log-entry log-entry--${level}`

  const time = document.createElement('time')
  time.className = 'log-entry__time'
  const now = new Date()
  time.dateTime = now.toISOString()
  time.textContent = now.toLocaleTimeString()

  const text = document.createElement('span')
  text.className = 'log-entry__text'
  text.textContent = message

  entry.append(time, text)
  list.prepend(entry)
}
