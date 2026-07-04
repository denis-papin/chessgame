// domain — pure board model for feature F0001. No DOM, no network.
// Operates only on the `Overview` model (the project's own board JSON, not FEN).

export type Castle = 'both' | 'short castle' | 'long castle' | 'none'

/** The board model served by `GET /start-game`. */
export interface Overview {
  /** 8 rows x 8 cols. board[0] = rank 8, board[7] = rank 1; col 0 = file a. */
  board: string[][]
  white: Castle
  black: Castle
}

/** An occupied square and the piece on it. */
export interface Cell {
  square: string
  piece: string
}

/** A rendered-board cell: its coordinate, colour, and optional piece. */
export interface BoardCell {
  square: string
  color: 'light' | 'dark'
  piece?: string
}

/** 64 cells in visual order: rank 8 first (top), rank 1 last (bottom). */
export type BoardModel = BoardCell[]

/** Map array indices to an algebraic square: file = a+col, rank = 8-row. */
function squareOf(row: number, col: number): string {
  const file = String.fromCharCode('a'.charCodeAt(0) + col)
  const rank = 8 - row
  return `${file}${rank}`
}

/**
 * Occupied squares only (rule F-3). Walks rows 0->7 (rank 8->1), cols 0->7
 * (file a->h), skipping empty cells.
 */
export function cells(ov: Overview): Cell[] {
  const out: Cell[] = []
  for (let r = 0; r < 8; r++) {
    for (let c = 0; c < 8; c++) {
      const piece = ov.board[r][c]
      if (piece) out.push({ square: squareOf(r, c), piece })
    }
  }
  return out
}

/**
 * Square colour (rule F-2). `a1` is dark; a square is dark when
 * (fileIndex + rankIndex) is even, with a=1..h=8 and rank 1..8.
 */
export function squareColor(square: string): 'light' | 'dark' {
  const fileIndex = square.charCodeAt(0) - 'a'.charCodeAt(0) + 1 // a=1..h=8
  const rankIndex = Number(square[1]) // 1..8
  return (fileIndex + rankIndex) % 2 === 0 ? 'dark' : 'light'
}

/**
 * Build the 64-cell board model from an `Overview` (rules F-1/F-2/F-3).
 * Order is visual: rank 8 first, rank 1 last, a1 at the bottom-left.
 */
export function buildBoard(ov: Overview): BoardModel {
  const model: BoardModel = []
  for (let r = 0; r < 8; r++) {
    for (let c = 0; c < 8; c++) {
      const square = squareOf(r, c)
      const piece = ov.board[r][c]
      model.push({ square, color: squareColor(square), ...(piece ? { piece } : {}) })
    }
  }
  return model
}
