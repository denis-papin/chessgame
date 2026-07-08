//! The `Square` type — a typed board coordinate replacing raw algebraic `&str`
//! in the internal geometry (F0002 technical refactor). Routines like
//! `legal_shape` / `accessible_squares` manipulate `Square` values; only the
//! request/response edge speaks the algebraic `a1..h8` wire form, so the public
//! contract is unchanged. This is a pure representation swap — no chess rule
//! lives here.

use std::fmt;

/// A board coordinate in the F0001 `Overview` order: `row` 0 = rank 8 …
/// `row` 7 = rank 1; `col` 0 = file `a` … `col` 7 = file `h`. Constructed from a
/// parsed algebraic string or directly from on-board indices, so a `Square`
/// always denotes a real cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square {
    /// Board row: 0 = rank 8 … 7 = rank 1.
    pub row: usize,
    /// Board column: 0 = file `a` … 7 = file `h`.
    pub col: usize,
}

impl Square {
    /// Build a `Square` from `(row, col)` board indices. Callers pass on-board
    /// indices (the geometry stays within `0..8`).
    pub const fn new(row: usize, col: usize) -> Square {
        Square { row, col }
    }

    /// Parse an algebraic square `a1..h8` into a `Square` in the F0001 board
    /// order (`a1` → row 7 col 0; rank 8 → row 0). `None` when malformed or out
    /// of range.
    pub fn parse(s: &str) -> Option<Square> {
        let b = s.as_bytes();
        if b.len() != 2 {
            return None;
        }
        let (file, rank) = (b[0], b[1]);
        if !(b'a'..=b'h').contains(&file) || !(b'1'..=b'8').contains(&rank) {
            return None;
        }
        let col = (file - b'a') as usize;
        let row = (8 - (rank - b'0')) as usize; // rank '1' → row 7, rank '8' → row 0
        Some(Square { row, col })
    }

    /// `true` when `s` is a square in `a1..h8` (rule **B-1** input validation).
    pub fn is_valid(s: &str) -> bool {
        Square::parse(s).is_some()
    }

    /// The algebraic form `a1..h8` — the inverse of [`Square::parse`].
    pub fn algebraic(self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = (b'a' + self.col as u8) as char;
        let rank = (b'0' + (8 - self.row as u8)) as char;
        write!(f, "{file}{rank}")
    }
}
