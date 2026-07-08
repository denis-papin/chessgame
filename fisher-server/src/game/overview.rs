//! The `Overview` board model and its standard / random builders.
//!
//! `Overview` is the project's own JSON board representation (F0001): an 8x8
//! grid of pieces plus per-colour castling availability. It is **not** FEN.
//! `board[0]` is rank 8, `board[7]` is rank 1; column 0 is file `a`, column 7 is
//! file `h`. Each cell is `Option<Piece>` — `None` (empty) or a typed
//! [`Piece`] (F0002 refactor). On the wire a cell stays its F0001 letter string
//! (`""` empty, uppercase White, lowercase Black), so the JSON contract is
//! unchanged.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Serialize, Serializer, ser::SerializeSeq};

use crate::game::piece::Piece;

/// One board cell: `None` (empty) or a typed piece.
pub type Cell = Option<Piece>;

/// The board model returned to the front end.
#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    /// 8 rows x 8 columns. `board[0]` = rank 8 ... `board[7]` = rank 1. Each
    /// cell is `None` (empty) or a [`Piece`]; it serialises to the F0001 letter.
    #[serde(serialize_with = "serialize_board")]
    pub board: Vec<Vec<Cell>>,
    /// White castling availability: `"both" | "short castle" | "long castle" | "none"`.
    pub white: String,
    /// Black castling availability.
    pub black: String,
}

/// Serialise the typed board to the F0001 wire shape — a `string[8][8]` where a
/// cell is `""` (empty) or the piece's single letter. Keeps the public JSON
/// contract identical to F0001 despite the internal `Piece` representation.
fn serialize_board<S: Serializer>(board: &[Vec<Cell>], serializer: S) -> Result<S::Ok, S::Error> {
    let mut rows = serializer.serialize_seq(Some(board.len()))?;
    for row in board {
        let cells: Vec<String> = row
            .iter()
            .map(|cell| cell.map_or(String::new(), |p| p.letter().to_string()))
            .collect();
        rows.serialize_element(&cells)?;
    }
    rows.end()
}

/// Read a builder cell literal (`""` empty, or a piece letter) into a [`Cell`].
/// The letters here are hard-coded and always valid, so an unknown letter is a
/// programming error and panics.
fn cell_of(s: &str) -> Cell {
    s.chars()
        .next()
        .map(|c| Piece::from_letter(c).expect("valid piece letter in a board builder"))
}

/// Build the standard starting position (rule B-2). Both sides keep "both".
pub fn standard_overview() -> Overview {
    let row = |cells: [&str; 8]| cells.into_iter().map(cell_of).collect::<Vec<Cell>>();
    let empty = || row(["", "", "", "", "", "", "", ""]);
    let board = vec![
        row(["r", "n", "b", "q", "k", "b", "n", "r"]), // rank 8
        row(["p", "p", "p", "p", "p", "p", "p", "p"]), // rank 7
        empty(),                                        // rank 6
        empty(),                                        // rank 5
        empty(),                                        // rank 4
        empty(),                                        // rank 3
        row(["P", "P", "P", "P", "P", "P", "P", "P"]), // rank 2
        row(["R", "N", "B", "Q", "K", "B", "N", "R"]), // rank 1
    ];
    Overview { board, white: "both".to_string(), black: "both".to_string() }
}

/// Build a random layout placing exactly `pieces` pieces of EACH colour at
/// random positions, drawn from a realistic army (rules B-3..B-8, B-10, B-11):
/// - exactly one king per colour (B-4),
/// - one piece per cell (B-5, enforced by the 2D array),
/// - no white pawn on rank 8 / no black pawn on rank 1 (B-6),
/// - no castling rights (B-7),
/// - per-colour material caps: <=1 queen, <=2 rooks, <=2 bishops, <=2 knights,
///   <=8 pawns; the caps sum to 16, so `pieces=16` forces the full army (B-10),
/// - a bishop pair sits on opposite-coloured squares (B-11).
///
/// `pieces` is assumed already validated to be in `2..=16` by the caller.
pub fn random_overview(pieces: u8) -> Overview {
    let mut board: Vec<Vec<Cell>> = vec![vec![None; 8]; 8];
    let mut rng = rand::thread_rng();

    place_army(&mut board, &mut rng, true, pieces); // white
    place_army(&mut board, &mut rng, false, pieces); // black

    Overview { board, white: "none".to_string(), black: "none".to_string() }
}

/// A board cell is dark when `(row + col)` is odd, so `board[7][0]` (`a1`) is
/// dark — the same parity as `squareColor` / rule F-2.
fn is_dark(row: usize, col: usize) -> bool {
    (row + col) % 2 == 1
}

/// The capped pool of non-king pieces for one colour (rule B-10): one queen,
/// two rooks, two bishops, two knights, eight pawns — 15 in total. The mandatory
/// king brings the per-colour maximum to 16.
fn non_king_pool(white: bool) -> Vec<Piece> {
    use Piece::*;
    let (q, r, b, n, p) = if white {
        (WhiteQueen, WhiteRook, WhiteBishop, WhiteKnight, WhitePawn)
    } else {
        (BlackQueen, BlackRook, BlackBishop, BlackKnight, BlackPawn)
    };
    let mut pool = vec![q, r, r, b, b, n, n];
    pool.extend(std::iter::repeat(p).take(8));
    pool
}

/// Place one colour's `pieces` (king + a capped, randomly-drawn army) onto empty
/// squares. The composition respects the per-type caps (B-10) and a bishop pair
/// is split across square colours (B-11).
fn place_army(board: &mut [Vec<Cell>], rng: &mut impl Rng, white: bool, pieces: u8) {
    use Piece::*;
    let king = if white { WhiteKing } else { BlackKing };
    let bishop = if white { WhiteBishop } else { BlackBishop };
    let pawn = if white { WhitePawn } else { BlackPawn };
    let forbid_pawn_row = if white { 0 } else { 7 }; // own pawn never on its far rank (B-6)

    // King first; it may sit anywhere.
    place_on(board, rng, king, |_, _| true);

    // Draw (pieces - 1) non-king pieces by shuffling the capped pool and taking
    // the front slice: this can never exceed a cap, and `pieces=16` takes all 15.
    let mut pool = non_king_pool(white);
    pool.shuffle(rng);
    let chosen = &pool[..(pieces as usize - 1)];

    // Bishops first, so the pair (B-11) can claim opposite-coloured squares before
    // the rest of the army fills the board.
    let bishops = chosen.iter().filter(|&&t| t == bishop).count();
    if bishops == 2 {
        place_on(board, rng, bishop, |r, c| is_dark(r, c)); // one dark
        place_on(board, rng, bishop, |r, c| !is_dark(r, c)); // one light
    } else if bishops == 1 {
        place_on(board, rng, bishop, |_, _| true);
    }

    // Then everything else; pawns avoid their own far rank (B-6).
    for &t in chosen {
        if t == bishop {
            continue; // already placed above
        }
        if t == pawn {
            place_on(board, rng, t, |r, _| r != forbid_pawn_row);
        } else {
            place_on(board, rng, t, |_, _| true);
        }
    }
}

/// Place `piece` on a uniformly-random empty cell that satisfies `allowed`. With
/// at most 32 pieces on 64 squares and the caps of B-10, a matching cell always
/// exists (the [Errors] 500 path is unreachable), so an empty candidate set is a
/// genuine invariant breach.
fn place_on(
    board: &mut [Vec<Cell>],
    rng: &mut impl Rng,
    piece: Piece,
    allowed: impl Fn(usize, usize) -> bool,
) {
    let candidates: Vec<(usize, usize)> = (0..8)
        .flat_map(|r| (0..8).map(move |c| (r, c)))
        .filter(|&(r, c)| board[r][c].is_none() && allowed(r, c))
        .collect();

    let &(r, c) = candidates
        .choose(rng)
        .expect("a valid empty square always exists under the B-10 caps");
    board[r][c] = Some(piece);
}
