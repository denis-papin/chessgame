//! The `Piece` enum — a typed chess piece replacing raw `char` / `String` board
//! cells (F0002 technical refactor). One variant per (colour, kind), so the code
//! matches on `Piece` / `Kind` instead of on letters. The board is a grid of
//! `Option<Piece>` (`None` = an empty square).
//!
//! On the wire a piece is still its F0001 letter — uppercase White, lowercase
//! Black — so `Piece` serialises to that single-letter string and the public
//! JSON contract is unchanged. This module is a pure representation swap: no
//! chess rule or logic lives here.

use std::fmt;

use serde::{Serialize, Serializer};

/// The six chess piece kinds, independent of colour. Lets the move geometry
/// match on `Kind::King` / `Kind::Pawn` … instead of on an uppercased letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

impl Kind {
    /// Parse a UCI promotion letter (`q r b n`, case-insensitive) into a `Kind`
    /// (F0003 rule B-6a). `None` for any other character — a promotion can only
    /// name a queen, rook, bishop, or knight.
    pub fn from_promo(c: char) -> Option<Kind> {
        Some(match c.to_ascii_lowercase() {
            'q' => Kind::Queen,
            'r' => Kind::Rook,
            'b' => Kind::Bishop,
            'n' => Kind::Knight,
            _ => return None,
        })
    }
}

/// A chess piece: its colour and kind fused into one variant. Uppercase letters
/// are White, lowercase Black — the F0001 `Overview` cell vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    WhiteKing,
    WhiteQueen,
    WhiteRook,
    WhiteBishop,
    WhiteKnight,
    WhitePawn,
    BlackKing,
    BlackQueen,
    BlackRook,
    BlackBishop,
    BlackKnight,
    BlackPawn,
}

impl Piece {
    /// `true` for a White (uppercase) piece.
    pub const fn is_white(self) -> bool {
        matches!(
            self,
            Piece::WhiteKing
                | Piece::WhiteQueen
                | Piece::WhiteRook
                | Piece::WhiteBishop
                | Piece::WhiteKnight
                | Piece::WhitePawn
        )
    }

    /// The black piece of a given `kind` — used for Black's promotion (F0003
    /// rule B-6a), where the promoted piece is always Black.
    pub const fn black(kind: Kind) -> Piece {
        match kind {
            Kind::King => Piece::BlackKing,
            Kind::Queen => Piece::BlackQueen,
            Kind::Rook => Piece::BlackRook,
            Kind::Bishop => Piece::BlackBishop,
            Kind::Knight => Piece::BlackKnight,
            Kind::Pawn => Piece::BlackPawn,
        }
    }

    /// The colour-independent kind (King, Queen, …).
    pub const fn kind(self) -> Kind {
        match self {
            Piece::WhiteKing | Piece::BlackKing => Kind::King,
            Piece::WhiteQueen | Piece::BlackQueen => Kind::Queen,
            Piece::WhiteRook | Piece::BlackRook => Kind::Rook,
            Piece::WhiteBishop | Piece::BlackBishop => Kind::Bishop,
            Piece::WhiteKnight | Piece::BlackKnight => Kind::Knight,
            Piece::WhitePawn | Piece::BlackPawn => Kind::Pawn,
        }
    }

    /// The single wire letter for this piece (uppercase White, lowercase Black).
    pub const fn letter(self) -> char {
        match self {
            Piece::WhiteKing => 'K',
            Piece::WhiteQueen => 'Q',
            Piece::WhiteRook => 'R',
            Piece::WhiteBishop => 'B',
            Piece::WhiteKnight => 'N',
            Piece::WhitePawn => 'P',
            Piece::BlackKing => 'k',
            Piece::BlackQueen => 'q',
            Piece::BlackRook => 'r',
            Piece::BlackBishop => 'b',
            Piece::BlackKnight => 'n',
            Piece::BlackPawn => 'p',
        }
    }

    /// Parse a wire letter into a `Piece`; `None` for any other character.
    pub fn from_letter(c: char) -> Option<Piece> {
        Some(match c {
            'K' => Piece::WhiteKing,
            'Q' => Piece::WhiteQueen,
            'R' => Piece::WhiteRook,
            'B' => Piece::WhiteBishop,
            'N' => Piece::WhiteKnight,
            'P' => Piece::WhitePawn,
            'k' => Piece::BlackKing,
            'q' => Piece::BlackQueen,
            'r' => Piece::BlackRook,
            'b' => Piece::BlackBishop,
            'n' => Piece::BlackKnight,
            'p' => Piece::BlackPawn,
            _ => return None,
        })
    }

    /// Parse a board-cell string into a cell: `""` → `None` (empty square), a
    /// single valid piece letter → `Some(piece)`, anything else → `None`.
    /// Callers that need to reject bad cells validate with [`is_valid_cell`]
    /// first, so here an unknown cell simply reads as empty.
    pub fn cell_from_str(s: &str) -> Option<Piece> {
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Piece::from_letter(c),
            _ => None,
        }
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.letter().encode_utf8(&mut [0u8; 1]))
    }
}

impl Serialize for Piece {
    /// Serialise to the single wire letter, so `Overview.board` cells,
    /// `MoveResponse.piece`, and `capture` keep the F0001 string vocabulary.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.letter().encode_utf8(&mut [0u8; 1]))
    }
}

/// `true` when `cell` is `""` (empty) or exactly one valid piece letter — the
/// F0001 cell vocabulary (F0002 rule **T-3**).
pub fn is_valid_cell(cell: &str) -> bool {
    if cell.is_empty() {
        return true;
    }
    let mut chars = cell.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if Piece::from_letter(c).is_some())
}
