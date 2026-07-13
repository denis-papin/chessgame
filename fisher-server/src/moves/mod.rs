//! moves — the authoritative move validator for feature F0002 (MOVE-A-PIECE).
//!
//! Pure chess geometry (`accessible_squares`, `legal_shape`, `path_clear`,
//! `board_after`, `is_in_check`, `validate_move`) plus the business delegate
//! `move_piece` that applies a validated White move and records any capture.
//!
//! The board is the F0001 `Overview` grid: `board[0]` is rank 8, `board[7]` is
//! rank 1; column 0 is file `a`, column 7 is file `h`. Uppercase letters are
//! White, lowercase are Black. All routines here are the constructive form of
//! the F0002 rules **B-2**–**B-15**; the enumeration and the per-check verdict
//! define the **same** pseudo-legal set (rule **B-13**).

use uuid::Uuid;

use crate::game::Registry;
use crate::game::overview::{Cell, Overview};
use crate::game::piece::{Kind, Piece};
use crate::game::square::Square;
use crate::proof_log::LogFeature;

/// A board is the `Overview.board` grid: rows of `Option<Piece>` cells.
type Board = [Vec<Cell>];

/// What sits on an accessible destination square (rule **B-11**).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The destination is vacant — a quiet move.
    Empty,
    /// The destination holds an enemy piece that would be taken.
    Capture(Piece),
}

/// A square the piece on `from` may legally reach, tagged by occupancy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessibleSquare {
    /// The reachable board coordinate.
    pub square: Square,
    /// `Empty` or `Capture(piece)`.
    pub target: Target,
}

/// Why a well-formed move was refused — the closed `IllegalReason` set
/// (rules **B-2**–**B-5**, **B-14**). Rendered into the `422` body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllegalReason {
    NotYourPiece,
    OwnPieceOnTarget,
    IllegalShape,
    PathBlocked,
    KingInCheck,
}

impl IllegalReason {
    /// The stable wire string carried in the `422 { reason }` body.
    pub const fn as_str(self) -> &'static str {
        match self {
            IllegalReason::NotYourPiece => "not your piece",
            IllegalReason::OwnPieceOnTarget => "own piece on target",
            IllegalReason::IllegalShape => "illegal shape",
            IllegalReason::PathBlocked => "path blocked",
            IllegalReason::KingInCheck => "king in check",
        }
    }
}

/// Outcome of `move_piece` (rule **B-8**): the handler derives the HTTP status
/// from this — `Applied` → `200`, `Illegal` → `422`, `UnknownGame` → `404`.
pub enum MoveOutcome {
    Applied { piece: Piece, capture: Option<Piece>, overview: Overview },
    Illegal(IllegalReason),
    UnknownGame,
}

// ---- coordinates ------------------------------------------------------------

/// The colour of a cell: `Some(true)` white, `Some(false)` black, `None` empty.
fn color_of(cell: Cell) -> Option<bool> {
    cell.map(|p| p.is_white())
}

// ---- accessible squares (rules B-11, B-12) ----------------------------------

/// Every square the piece on `from` may legally reach, each tagged `Empty` or
/// `Capture` (rules **B-11**, **B-12**). Runs for either colour — White to
/// enumerate its moves, Black during check detection (rule **B-15**). Off-board
/// and friendly-occupied squares are excluded (rule **B-3**). Pure.
pub fn accessible_squares(board: &Board, from: Square) -> Vec<AccessibleSquare> {
    let mut out = Vec::new();
    let (r, c) = (from.row, from.col);
    let piece = match board[r][c] {
        Some(p) => p,
        None => return out, // empty source has no moves
    };
    let white = piece.is_white();

    // Classify a single destination for a stepper (king/knight): `None` off-board
    // or friendly; `Empty`/`Capture` otherwise.
    let occ = |nr: i32, nc: i32| -> Option<AccessibleSquare> {
        if !(0..8).contains(&nr) || !(0..8).contains(&nc) {
            return None;
        }
        let (ur, uc) = (nr as usize, nc as usize);
        match board[ur][uc] {
            None => Some(AccessibleSquare { square: Square::new(ur, uc), target: Target::Empty }),
            Some(p) if p.is_white() != white => Some(AccessibleSquare {
                square: Square::new(ur, uc),
                target: Target::Capture(p),
            }),
            Some(_) => None, // friendly → excluded
        }
    };

    // Walk one ray until the first occupied square (rule **B-5** constructive):
    // enemy there is a `Capture` that ends the ray, friendly ends it and is dropped.
    let ray = |dr: i32, dc: i32, out: &mut Vec<AccessibleSquare>| {
        let (mut nr, mut nc) = (r as i32 + dr, c as i32 + dc);
        while (0..8).contains(&nr) && (0..8).contains(&nc) {
            let (ur, uc) = (nr as usize, nc as usize);
            match board[ur][uc] {
                None => out.push(AccessibleSquare { square: Square::new(ur, uc), target: Target::Empty }),
                Some(p) if p.is_white() != white => {
                    out.push(AccessibleSquare { square: Square::new(ur, uc), target: Target::Capture(p) });
                    break;
                }
                Some(_) => break,
            }
            nr += dr;
            nc += dc;
        }
    };

    let (ri, ci) = (r as i32, c as i32);
    match piece.kind() {
        Kind::King => {
            for (dr, dc) in [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)] {
                if let Some(a) = occ(ri + dr, ci + dc) {
                    out.push(a);
                }
            }
        }
        Kind::Knight => {
            for (dr, dc) in [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)] {
                if let Some(a) = occ(ri + dr, ci + dc) {
                    out.push(a);
                }
            }
        }
        Kind::Rook => {
            for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                ray(dr, dc, &mut out);
            }
        }
        Kind::Bishop => {
            for (dr, dc) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
                ray(dr, dc, &mut out);
            }
        }
        Kind::Queen => {
            for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)] {
                ray(dr, dc, &mut out);
            }
        }
        Kind::Pawn => {
            // Forward is toward the far rank for the colour (White up = -row).
            let dir: i32 = if white { -1 } else { 1 };
            let base_row: i32 = if white { 6 } else { 1 };
            let one = ri + dir;
            if (0..8).contains(&one) && board[one as usize][c].is_none() {
                out.push(AccessibleSquare { square: Square::new(one as usize, c), target: Target::Empty });
                let two = ri + 2 * dir;
                if ri == base_row && (0..8).contains(&two) && board[two as usize][c].is_none() {
                    out.push(AccessibleSquare { square: Square::new(two as usize, c), target: Target::Empty });
                }
            }
            // Diagonals count only as a capture of an enemy (never onto empty).
            for dc in [-1, 1] {
                let (nr, nc) = (one, ci + dc);
                if (0..8).contains(&nr) && (0..8).contains(&nc) {
                    let (ur, uc) = (nr as usize, nc as usize);
                    if let Some(p) = board[ur][uc] {
                        if p.is_white() != white {
                            out.push(AccessibleSquare { square: Square::new(ur, uc), target: Target::Capture(p) });
                        }
                    }
                }
            }
        }
    }
    out
}

// ---- shape & path (rules B-4, B-5, B-12) ------------------------------------

/// Pure per-piece geometry (rule **B-4**, and the pawn cases of **B-12**),
/// testing shape while **ignoring** sliding blockers — a pawn diagonal counts as
/// a shape only when it lands on an enemy (rule **B-12**).
pub fn legal_shape(piece: Piece, from: Square, to: Square, board: &Board) -> bool {
    let (fr, fc) = (from.row, from.col);
    let (tr, tc) = (to.row, to.col);
    let dr = tr as i32 - fr as i32;
    let dc = tc as i32 - fc as i32;
    let white = piece.is_white();

    match piece.kind() {
        Kind::King => dr.abs().max(dc.abs()) == 1,
        Kind::Knight => {
            let (a, b) = (dr.abs(), dc.abs());
            (a == 1 && b == 2) || (a == 2 && b == 1)
        }
        Kind::Rook => (dr == 0) ^ (dc == 0),
        Kind::Bishop => dr.abs() == dc.abs() && dr != 0,
        Kind::Queen => ((dr == 0) ^ (dc == 0)) || (dr.abs() == dc.abs() && dr != 0),
        Kind::Pawn => {
            let dir = if white { -1 } else { 1 };
            let base_row = if white { 6 } else { 1 };
            let target_empty = board[tr][tc].is_none();
            if dc == 0 && dr == dir && target_empty {
                true
            } else if dc == 0 && dr == 2 * dir && fr as i32 == base_row && target_empty {
                true
            } else if dc.abs() == 1 && dr == dir {
                matches!(board[tr][tc], Some(p) if p.is_white() != white)
            } else {
                false
            }
        }
    }
}

/// Every square strictly between `from` and `to` is empty (rule **B-5**);
/// trivially `true` for knight/king (no square strictly between). Pure.
pub fn path_clear(board: &Board, from: Square, to: Square) -> bool {
    let (fr, fc) = (from.row, from.col);
    let (tr, tc) = (to.row, to.col);
    let dr = (tr as i32 - fr as i32).signum();
    let dc = (tc as i32 - fc as i32).signum();
    let (mut r, mut c) = (fr as i32 + dr, fc as i32 + dc);
    while (r, c) != (tr as i32, tc as i32) {
        if !(0..8).contains(&r) || !(0..8).contains(&c) {
            break; // not colinear — nothing strictly between along a line
        }
        if board[r as usize][c as usize].is_some() {
            return false;
        }
        r += dr;
        c += dc;
    }
    true
}

// ---- king safety (rules B-14, B-15) -----------------------------------------

/// The board with the move applied — piece moved, any capture removed (the
/// position `is_in_check` tests, rule **B-14**, and the apply step writes,
/// rule **B-7**). Pure.
pub fn board_after(board: &Board, from: Square, to: Square) -> Vec<Vec<Cell>> {
    let mut b: Vec<Vec<Cell>> = board.to_vec();
    let piece = b[from.row][from.col].take();
    b[to.row][to.col] = piece;
    b
}

/// `true` when the `white` side's king is the `target` of any opposing piece's
/// `accessible_squares` on `board` (rule **B-15**). No king found → `false`. Pure.
pub fn is_in_check(board: &Board, white: bool) -> bool {
    let king = if white { Piece::WhiteKing } else { Piece::BlackKing };
    let mut king_sq = None;
    for r in 0..8 {
        for c in 0..8 {
            if board[r][c] == Some(king) {
                king_sq = Some(Square::new(r, c));
            }
        }
    }
    let king_sq = match king_sq {
        Some(s) => s,
        None => return false,
    };
    for r in 0..8 {
        for c in 0..8 {
            if let Some(w) = color_of(board[r][c])
                && w != white
                && accessible_squares(board, Square::new(r, c)).iter().any(|a| a.square == king_sq)
            {
                return true;
            }
        }
    }
    false
}

// ---- validate & apply (rules B-2..B-14) -------------------------------------

/// Full legality check (rules **B-2**–**B-6**, **B-12**–**B-14**). Returns the
/// captured black piece (if any) on success, or the precise `IllegalReason`.
/// Pseudo-legality is membership in `accessible_squares` (rule **B-13**); the
/// king-safety gate (`is_in_check` over `board_after`) is the final step. Pure.
pub fn validate_move(board: &Board, from: Square, to: Square) -> Result<Option<Piece>, IllegalReason> {
    let src = board[from.row][from.col];

    // B-2 — source must carry a white piece.
    let (piece, white) = match src {
        Some(p) if p.is_white() => (p, true),
        _ => return Err(IllegalReason::NotYourPiece),
    };
    // B-3 — target must not hold a white piece.
    if matches!(board[to.row][to.col], Some(p) if p.is_white()) {
        return Err(IllegalReason::OwnPieceOnTarget);
    }

    // B-4/B-5/B-12/B-13 — pseudo-legality is membership in accessible_squares.
    let acc = accessible_squares(board, from);
    let capture = match acc.iter().find(|a| a.square == to) {
        Some(a) => match a.target {
            Target::Empty => None,
            Target::Capture(p) => Some(p),
        },
        None => {
            // Not pseudo-legal: shape fits but the path is blocked (rule **B-5**),
            // otherwise the shape itself is wrong (rule **B-4**).
            if legal_shape(piece, from, to, board) && !path_clear(board, from, to) {
                return Err(IllegalReason::PathBlocked);
            }
            return Err(IllegalReason::IllegalShape);
        }
    };

    // B-14 — king safety: the move must not leave White's king in check.
    if is_in_check(&board_after(board, from, to), white) {
        return Err(IllegalReason::KingInCheck);
    }

    Ok(capture)
}

/// Business delegate (rule **B-8**): look up the game, validate the move against
/// its stored board, and on success apply it — clear `from`, write the piece onto
/// `to`, append any captured black piece to the game's taken list (rule **B-7**)
/// — then return the outcome. An illegal move leaves the game unchanged.
pub fn move_piece(registry: &Registry, uuid: &str, from: Square, to: Square, session: &str, tracking: &str) -> MoveOutcome {
    let id = match Uuid::parse_str(uuid) {
        Ok(id) => id,
        Err(_) => return MoveOutcome::UnknownGame,
    };

    let mut guard = registry.lock().expect("registry mutex poisoned");
    let game = match guard.get_mut(&id) {
        Some(g) => g,
        None => return MoveOutcome::UnknownGame,
    };

    // Validate against a copy so the borrow is free when we apply below.
    let board = game.overview.board.clone();
    match validate_move(&board, from, to) {
        Err(reason) => {
            // B-7 — an illegal move changes nothing.
            log_warn_f!(LogFeature::MoveAPiece.as_str(), session, tracking, uuid = %id, from = %from, to = %to, reason = reason.as_str(), "move is illegal");
            MoveOutcome::Illegal(reason)
        }
        Ok(capture) => {
            let piece = board[from.row][from.col].expect("validated source carries a piece");

            // Invariant / Rule Check (rule 6): the move satisfied B-2..B-14 —
            // pseudo-legal (membership in accessible_squares) and king-safe.
            log_info_f!(LogFeature::MoveAPiece.as_str(), session, tracking, uuid = %id, from = %from, to = %to, piece = %piece, legal = true, "move legality validated");

            // Business Decision (rule 4): the outcome path — a capture or a quiet move.
            let move_kind = if capture.is_some() { "capture" } else { "quiet" };
            log_info_f!(LogFeature::MoveAPiece.as_str(), session, tracking, uuid = %id, move_kind = move_kind, capture = ?capture, "move path selected");

            // B-7 — apply the move to the stored board.
            game.overview.board = board_after(&board, from, to);

            // B-2/B-3 (F0004) — a White pawn reaching rank 8 (row 0) is replaced by
            // a White Queen. The apply step is the only change; validation is
            // untouched. Mirrors the Black promotion in engine::apply_black_move.
            if piece == Piece::WhitePawn && to.row == 0 {
                game.overview.board[to.row][to.col] = Some(Piece::WhiteQueen);
                // Business Decision (rule 4): the move routed onto the promotion
                // branch — a White pawn reached rank 8 and became a Queen (F0004
                // B-2/B-3). Emitted on the WHITE-PAWN-PROMOTION stream.
                log_info_f!(LogFeature::WhitePawnPromotion.as_str(), session, tracking, uuid = %id, from = %from, to = %to, promoted_to = "Q", "white pawn promoted to queen");
            }

            // Business Milestone (rule 7): the board position is transformed.
            log_info_f!(LogFeature::MoveAPiece.as_str(), session, tracking, uuid = %id, from = %from, to = %to, piece = %piece, "board updated");

            // B-7 — record any captured black piece on the game's taken list.
            if let Some(p) = capture {
                game.taken.push(p);
            }
            // State Change (rule 5): the game's stored Overview (+ taken list) is
            // now updated in the registry, keyed by uuid.
            log_info_f!(LogFeature::MoveAPiece.as_str(), session, tracking, uuid = %id, from = %from, to = %to, capture = ?capture, taken_count = game.taken.len(), "move recorded");

            MoveOutcome::Applied { piece, capture, overview: game.overview.clone() }
        }
    }
}

// ---- unit tests: accessible_squares & is_in_check (rules B-11..B-15) ---------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a board from 8 rank-8-first rows of space-separated cells, `.` empty.
    fn board(rows: [&str; 8]) -> Vec<Vec<Cell>> {
        rows.iter()
            .map(|row| {
                row.split(' ')
                    .map(|c| if c == "." { None } else { Piece::from_letter(c.chars().next().unwrap()) })
                    .collect::<Vec<Cell>>()
            })
            .collect()
    }

    fn standard() -> Vec<Vec<Cell>> {
        board([
            "r n b q k b n r",
            "p p p p p p p p",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            "P P P P P P P P",
            "R N B Q K B N R",
        ])
    }

    /// Parse an algebraic square in a test — always a valid coordinate here.
    fn sq(s: &str) -> Square {
        Square::parse(s).expect("valid test square")
    }

    /// Collect accessible squares as `(square, capture-letter-or-empty)` pairs, sorted.
    fn acc(b: &Board, from: &str) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = accessible_squares(b, sq(from))
            .into_iter()
            .map(|a| {
                let cap = match a.target {
                    Target::Empty => String::new(),
                    Target::Capture(p) => p.letter().to_string(),
                };
                (a.square.algebraic(), cap)
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn pawn_double_push_from_base_rank() {
        // d2 (P): d3 Empty, d4 Empty; diagonals c3/e3 empty → excluded (B-12).
        assert_eq!(acc(&standard(), "d2"), vec![
            ("d3".to_string(), String::new()),
            ("d4".to_string(), String::new()),
        ]);
    }

    #[test]
    fn knight_from_b1_drops_friendly() {
        // b1 (N): a3 Empty, c3 Empty (d2 is friendly → dropped) (B-11/B-12).
        assert_eq!(acc(&standard(), "b1"), vec![
            ("a3".to_string(), String::new()),
            ("c3".to_string(), String::new()),
        ]);
    }

    #[test]
    fn rook_and_king_boxed_in_have_no_moves() {
        // a1 (R): a2 friendly ends file, b1 friendly ends rank → ∅.
        assert!(acc(&standard(), "a1").is_empty());
        // e1 (K): every neighbour holds a friendly piece → ∅.
        assert!(acc(&standard(), "e1").is_empty());
    }

    #[test]
    fn pawn_capture_and_empty_tags() {
        // Capture board: White P e4, Black p d5, kings e1/e8.
        let b = board([
            ". . . . k . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . p . . . .",
            ". . . . P . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . K . . .",
        ]);
        // e4 (P): e5 Empty, d5 Capture(p); f5 diagonal empty → excluded; not on
        // rank 2 → no double push (B-12, B-6).
        assert_eq!(acc(&b, "e4"), vec![
            ("d5".to_string(), "p".to_string()),
            ("e5".to_string(), String::new()),
        ]);
    }

    #[test]
    fn rook_ray_stops_at_capture() {
        // White R a1, Black p a5, else empty: ray up the file stops at a5.
        let b = board([
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            "p . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            "R . . . . . . .",
        ]);
        let got = acc(&b, "a1");
        assert!(got.contains(&("a2".to_string(), String::new())));
        assert!(got.contains(&("a4".to_string(), String::new())));
        assert!(got.contains(&("a5".to_string(), "p".to_string())));
        // The ray stops at a5 — a6/a7/a8 are not reachable.
        assert!(!got.iter().any(|(s, _)| s == "a6" || s == "a7" || s == "a8"));
        // The rank ray runs b1..h1 over empty squares.
        assert!(got.contains(&("h1".to_string(), String::new())));
    }

    #[test]
    fn black_pawn_moves_downward() {
        // A lone black pawn on d7 pushes to d6/d5 (forward = toward rank 1).
        let b = board([
            ". . . . k . . .",
            ". . . p . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . K . . .",
        ]);
        assert_eq!(acc(&b, "d7"), vec![
            ("d5".to_string(), String::new()),
            ("d6".to_string(), String::new()),
        ]);
    }

    #[test]
    fn check_by_black_rook_along_file() {
        // White K e1, Black r e8 down an open file → White is in check (B-15).
        let b = board([
            "k . . . r . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . K . . .",
        ]);
        assert!(is_in_check(&b, true), "rook on the open e-file checks the king (B-15)");
    }

    #[test]
    fn check_by_black_knight() {
        // White K e1, Black n f3 attacks e1 (knight L) → in check (B-15).
        let b = board([
            "k . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . n . .",
            ". . . . . . . .",
            ". . . . K . . .",
        ]);
        assert!(is_in_check(&b, true), "knight on f3 checks the king (B-15)");
    }

    #[test]
    fn check_by_black_pawn_diagonal_not_straight() {
        // Black pawn d2 attacks the white king on e1 diagonally (forward-down).
        let checking = board([
            "k . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . p . . . .",
            ". . . . K . . .",
        ]);
        assert!(is_in_check(&checking, true), "black pawn checks along its diagonal (B-15)");

        // A black pawn directly in front (e2) does NOT check — a push needs an
        // empty square, so it never attacks straight ahead (B-15).
        let safe = board([
            "k . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . p . . .",
            ". . . . K . . .",
        ]);
        assert!(!is_in_check(&safe, true), "a pawn straight ahead is not a check (B-15)");
    }

    #[test]
    fn safe_position_is_not_in_check() {
        // Standard opening: White's king is not in check.
        assert!(!is_in_check(&standard(), true));
    }

    #[test]
    fn validate_pinned_bishop_is_king_in_check() {
        // Pin board: White K e1, White B e2, Black r e8, Black k a8.
        let b = board([
            "k . . . r . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . B . . .",
            ". . . . K . . .",
        ]);
        // Moving the pinned bishop opens the file → king in check (B-14).
        assert_eq!(validate_move(&b, sq("e2"), sq("d3")), Err(IllegalReason::KingInCheck));
        // The king may step off the file to a safe square (B-14).
        assert_eq!(validate_move(&b, sq("e1"), sq("f1")), Ok(None));
    }

    // ---- F0004: white pawn promotion at the apply step (rules B-2, B-3, B-6) --

    /// Build a one-game registry from `board` and return it with the game `uuid`.
    fn registry_with(board: Vec<Vec<Cell>>) -> (Registry, String) {
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};

        use crate::game::{Game, Mode};

        let uuid = Uuid::new_v4();
        let overview = Overview { board, white: "both".to_string(), black: "both".to_string() };
        let game = Game { uuid, mode: Mode::Standard, overview, taken: Vec::new() };
        let mut map = HashMap::new();
        map.insert(uuid, game);
        (Arc::new(Mutex::new(map)), uuid.to_string())
    }

    #[test]
    fn white_pawn_push_to_rank_8_promotes_to_queen() {
        // White P e7, White K e1, Black r d8, Black k a8. Push e7→e8 promotes.
        let b = board([
            "k . . r . . . .",
            ". . . . P . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . K . . .",
        ]);
        let (registry, uuid) = registry_with(b);
        match move_piece(&registry, &uuid, sq("e7"), sq("e8"), "s", "t") {
            MoveOutcome::Applied { piece, capture, overview } => {
                // B-5 — the response echoes the mover pawn, not the queen.
                assert_eq!(piece, Piece::WhitePawn, "piece stays WhitePawn (B-5)");
                assert_eq!(capture, None, "a promotion push takes nothing (B-5)");
                // B-2/B-3 — the destination cell (rank 8 = row 0, file e = col 4) is a queen.
                assert_eq!(overview.board[0][4], Some(Piece::WhiteQueen), "e8 holds a White Queen (B-2/B-3)");
                assert_eq!(overview.board[1][4], None, "the pawn left e7 (B-7)");
            }
            other => panic!("promotion push must be applied, got a non-Applied outcome: {}", label(&other)),
        }
    }

    #[test]
    fn white_pawn_diagonal_capture_to_rank_8_promotes_to_queen() {
        // Same board: e7 pawn captures the black rook on d8 and promotes.
        let b = board([
            "k . . r . . . .",
            ". . . . P . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . K . . .",
        ]);
        let (registry, uuid) = registry_with(b);
        match move_piece(&registry, &uuid, sq("e7"), sq("d8"), "s", "t") {
            MoveOutcome::Applied { piece, capture, overview } => {
                // B-5 — the mover is the pawn; the taken rook is still reported.
                assert_eq!(piece, Piece::WhitePawn, "piece stays WhitePawn (B-5)");
                assert_eq!(capture, Some(Piece::BlackRook), "the captured rook is reported (B-5)");
                // B-3/B-4 — the capture square (d8 = row 0, col 3) now holds a queen.
                assert_eq!(overview.board[0][3], Some(Piece::WhiteQueen), "d8 holds a White Queen (B-3/B-4)");
            }
            other => panic!("promotion capture must be applied, got: {}", label(&other)),
        }
    }

    #[test]
    fn white_pawn_move_off_rank_8_is_not_promoted() {
        // Standard opening: d2→d4 is a normal push — no promotion off rank 8 (B-6).
        let (registry, uuid) = registry_with(standard());
        match move_piece(&registry, &uuid, sq("d2"), sq("d4"), "s", "t") {
            MoveOutcome::Applied { piece, overview, .. } => {
                assert_eq!(piece, Piece::WhitePawn, "piece stays WhitePawn");
                // d4 = row 4, col 3 — still a pawn, never a queen (B-6).
                assert_eq!(overview.board[4][3], Some(Piece::WhitePawn), "d4 holds a pawn, not a queen (B-6)");
            }
            other => panic!("normal pawn push must be applied, got: {}", label(&other)),
        }
    }

    /// A short label for a `MoveOutcome`, for panic messages in the tests above.
    fn label(outcome: &MoveOutcome) -> String {
        match outcome {
            MoveOutcome::Applied { .. } => "Applied".to_string(),
            MoveOutcome::Illegal(r) => format!("Illegal({})", r.as_str()),
            MoveOutcome::UnknownGame => "UnknownGame".to_string(),
        }
    }
}
