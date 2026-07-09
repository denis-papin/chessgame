//! engine — the Stockfish opponent reply for feature F0003 (PLAY-WITH-STOCKFISH).
//!
//! `opponent_move` computes **Black's** reply with the real Stockfish engine over
//! its REST API and scores the result. The pieces:
//!
//! - `to_fen` — serialize the stored `Overview` to a Black-to-move FEN (rule B-4).
//!   FEN lives **only** here; the app contract stays on the `Overview` model.
//! - `best_move` — call `GET /bestmove` and read `result.bestmove` plus the
//!   deepest line's `score` into `EngineReply { best, mate_in }` (rule B-5).
//! - `parse_uci` / `apply_black_move` — parse the UCI move and apply it to the
//!   board, recording any captured white piece (rules B-5, B-6a, B-8).
//! - `game_status` — score the position after the reply (rule B-6):
//!   `white-in-checkmate` from the engine's `mate 1`, else `white-in-check` from
//!   the server's own `is_in_check` (Stockfish reports no plain check), else
//!   `move`. Black's own game-over (`(none)`) is split by `is_in_check` (rule B-3).
//!
//! Move existence for Black — and whether Black's reply mates White — is the
//! **engine's** call; this module runs no legal-move generator (rule B-7).

use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

use crate::game::Registry;
use crate::game::overview::{Cell, Overview};
use crate::game::piece::{Kind, Piece};
use crate::game::square::Square;
use crate::moves::{board_after, is_in_check};
use crate::proof_log::LogFeature;

/// Default Stockfish REST base URL (rule B-5). One place, overridable by env so
/// the harness can point at another engine; never scattered across call sites.
const DEFAULT_ENGINE_BASE: &str = "http://localhost:4000";

/// Fixed search depth for the engine call (rule B-5). Engine-difficulty as a
/// game setting is out of scope, so this stays a constant.
const DEPTH: u32 = 12;

/// The engine base URL, from `ENGINE_BASE` or the default.
fn engine_base() -> String {
    std::env::var("ENGINE_BASE").unwrap_or_else(|_| DEFAULT_ENGINE_BASE.to_string())
}

// ---- game status (rules B-3, B-6) -------------------------------------------

/// The game state carried in the reply (rule B-6). `white-in-stalemate` and
/// `draw` are reserved but never emitted here (see F0003 Out of scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    /// An ordinary reply; White is to move and not in check.
    Move,
    /// Black's reply gives check — White is in check (server-decided).
    WhiteInCheck,
    /// Black's reply mates White (the engine scored it `mate 1`).
    WhiteInCheckmate,
    /// Black (the engine's side) has no legal move and is in check.
    BlackInCheckmate,
    /// Black has no legal move and is not in check — a draw.
    BlackInStalemate,
}

impl GameStatus {
    /// The stable wire string carried in the `status` field.
    pub const fn as_str(self) -> &'static str {
        match self {
            GameStatus::Move => "move",
            GameStatus::WhiteInCheck => "white-in-check",
            GameStatus::WhiteInCheckmate => "white-in-checkmate",
            GameStatus::BlackInCheckmate => "black-in-checkmate",
            GameStatus::BlackInStalemate => "black-in-stalemate",
        }
    }
}

impl Serialize for GameStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Score the position **after** Black's reply for the side now to move (White),
/// rule B-6. `mate_in == Some(1)` → `white-in-checkmate` (the engine's own
/// verdict). Otherwise `is_in_check(board, white)` distinguishes `white-in-check`
/// from an ordinary `move` — the one status the server computes itself, because
/// Stockfish reports no plain-check flag (rule B-7). Pure.
pub fn game_status(board: &[Vec<Cell>], mate_in: Option<i32>) -> GameStatus {
    if mate_in == Some(1) {
        GameStatus::WhiteInCheckmate
    } else if is_in_check(board, true) {
        GameStatus::WhiteInCheck
    } else {
        GameStatus::Move
    }
}

// ---- FEN serialization (rule B-4) -------------------------------------------

/// Serialize `overview` to a FEN string with **Black to move** — the only side
/// the engine is ever asked about (rule B-4). Castling `-`, en passant `-`,
/// halfmove `0`, fullmove `1`. Pure.
pub fn to_fen(overview: &Overview) -> String {
    let mut ranks = Vec::with_capacity(8);
    for row in &overview.board {
        let mut rank = String::new();
        let mut empty = 0u32;
        for cell in row {
            match cell {
                None => empty += 1,
                Some(piece) => {
                    if empty > 0 {
                        rank.push_str(&empty.to_string());
                        empty = 0;
                    }
                    rank.push(piece.letter());
                }
            }
        }
        if empty > 0 {
            rank.push_str(&empty.to_string());
        }
        ranks.push(rank);
    }
    format!("{} b - - 0 1", ranks.join("/"))
}

// ---- UCI parse & apply (rules B-5, B-6a, B-8) -------------------------------

/// A parsed UCI long-algebraic move: two squares and an optional promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UciMove {
    pub from: Square,
    pub to: Square,
    /// Promotion target when a pawn promotes — a colourless `Kind` (the side is
    /// implied by the mover, rule B-6a). `None` for an ordinary move.
    pub promo: Option<Kind>,
}

/// Split a UCI move (`"c7c5"`, `"a2a1q"`) into `from`, `to`, and an optional
/// promotion `Kind` (rule B-5). `None` when malformed. Pure.
pub fn parse_uci(s: &str) -> Option<UciMove> {
    if s.len() < 4 {
        return None;
    }
    let from = Square::parse(&s[0..2])?;
    let to = Square::parse(&s[2..4])?;
    let promo = s.chars().nth(4).and_then(Kind::from_promo);
    Some(UciMove { from, to, promo })
}

/// The board with Black's move applied — `from` cleared, `to` set to the moving
/// piece (or the promoted black piece, rule B-6a) — plus any captured **white**
/// piece (rule B-8). Built on `board_after`. Pure.
pub fn apply_black_move(board: &[Vec<Cell>], uci: &UciMove) -> (Vec<Vec<Cell>>, Option<Piece>) {
    // Whatever stands on the target — a white piece for a legal black capture.
    let capture = board[uci.to.row][uci.to.col];
    let mut next = board_after(board, uci.from, uci.to);
    if let Some(kind) = uci.promo {
        // The promoted piece is always Black (this module only plays Black).
        next[uci.to.row][uci.to.col] = Some(Piece::black(kind));
    }
    (next, capture)
}

// ---- engine REST call (rule B-5) --------------------------------------------

/// The parsed engine reply: the best move (or `None` for `"(none)"`), and the
/// deepest line's mate distance (`Some(n)` for a `mate` score, `None` for `cp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineReply {
    pub best: Option<UciMove>,
    pub mate_in: Option<i32>,
}

/// An engine transport or protocol failure — surfaces as `502` (rule B-9).
#[derive(Debug)]
pub struct EngineError;

#[derive(Deserialize)]
struct EngineResponse {
    result: EngineResult,
}

#[derive(Deserialize)]
struct EngineResult {
    bestmove: String,
    #[serde(default)]
    info: Vec<InfoLine>,
}

#[derive(Deserialize)]
struct InfoLine {
    depth: Option<i32>,
    score: Option<Score>,
}

#[derive(Deserialize)]
struct Score {
    unit: String,
    value: i32,
}

/// The mate distance of the deepest scored PV line — `Some(n)` when its
/// `score.unit == "mate"`, else `None` (a `cp` score). This is what reveals a
/// White-mating reply (`Some(1)`, rule B-6).
fn deepest_mate_in(info: &[InfoLine]) -> Option<i32> {
    let best_line = info
        .iter()
        .filter(|l| l.depth.is_some() && l.score.is_some())
        .max_by_key(|l| l.depth.unwrap());
    match best_line.and_then(|l| l.score.as_ref()) {
        Some(s) if s.unit == "mate" => Some(s.value),
        _ => None,
    }
}

/// Call `GET {ENGINE_BASE}/bestmove?fen=…&depth=…` and build `EngineReply`
/// (rule B-5). `bestmove == "(none)"` → `best: None` (a legal game-over signal,
/// rule B-3, not an error). Any transport or protocol failure → `EngineError`.
pub async fn best_move(fen: &str) -> Result<EngineReply, EngineError> {
    let url = format!("{}/bestmove", engine_base());
    let resp = reqwest::Client::new()
        .get(&url)
        .query(&[("fen", fen), ("depth", &DEPTH.to_string())])
        .send()
        .await
        .map_err(|_| EngineError)?;
    if !resp.status().is_success() {
        return Err(EngineError);
    }
    let body: EngineResponse = resp.json().await.map_err(|_| EngineError)?;

    let best = if body.result.bestmove == "(none)" {
        None
    } else {
        Some(parse_uci(&body.result.bestmove).ok_or(EngineError)?)
    };
    Ok(EngineReply {
        best,
        mate_in: deepest_mate_in(&body.result.info),
    })
}

// ---- the reply outcome & business delegate (rules B-1..B-9) -----------------

/// The reply body (rule B-8). `from`/`to`/`piece`/`capture` are `null` on a
/// game-over result (Black had no move, rule B-3).
#[derive(Debug, Serialize)]
pub struct OpponentReply {
    pub from: Option<String>,
    pub to: Option<String>,
    pub piece: Option<Piece>,
    pub capture: Option<Piece>,
    pub status: GameStatus,
    pub overview: Overview,
}

/// Outcome of `opponent_move` — the handler derives the HTTP status: `Ok` → `200`,
/// `UnknownGame` → `404`, `EngineUnavailable` → `502` (rules B-1, B-9).
pub enum OpponentOutcome {
    Ok(Box<OpponentReply>),
    UnknownGame,
    EngineUnavailable,
}

/// Business delegate (rules B-1–B-9): load the game's `Overview`, ask the engine
/// for Black's reply, and either report the black-side game-over (rule B-3) or
/// apply the reply, record any capture, score the result, and return it. The
/// engine is asked exactly once, for Black (rule B-4).
pub async fn opponent_move(registry: &Registry, uuid: &str, session: &str, tracking: &str) -> OpponentOutcome {
    let id = match Uuid::parse_str(uuid) {
        Ok(id) => id,
        Err(_) => return OpponentOutcome::UnknownGame,
    };

    // Load the stored position, then release the lock before the engine await
    // (never hold the registry mutex across `.await`).
    let overview = {
        let guard = registry.lock().expect("registry mutex poisoned");
        match guard.get(&id) {
            Some(game) => game.overview.clone(),
            None => return OpponentOutcome::UnknownGame,
        }
    };

    // B-4 — serialize to a Black-to-move FEN. Business Milestone (rule 7).
    let fen = to_fen(&overview);
    log_info_f!(LogFeature::PlayWithStockfish.as_str(), session, tracking, uuid = %id, "position serialized to FEN (Black to move)");

    // External Boundary — request (rule 3): before the call to Stockfish.
    log_info_f!(LogFeature::PlayWithStockfish.as_str(), session, tracking, uuid = %id, depth = DEPTH, "→ requesting Black's reply from Stockfish");

    // B-5 — one engine call, for Black.
    let reply = match best_move(&fen).await {
        Ok(reply) => reply,
        Err(_) => {
            // External Boundary — response failure (rule 3) + Error / Exception (rule 8).
            log_error_f!(LogFeature::PlayWithStockfish.as_str(), session, tracking, uuid = %id, error = "engine unavailable", "← engine call failed");
            return OpponentOutcome::EngineUnavailable;
        }
    };

    // External Boundary — response (rule 3): the engine answered.
    let bestmove = match reply.best {
        Some(m) => format!("{}{}", m.from, m.to),
        None => "(none)".to_string(),
    };
    log_info_f!(LogFeature::PlayWithStockfish.as_str(), session, tracking, uuid = %id, bestmove = %bestmove, mate_in = ?reply.mate_in, "← Stockfish responded");

    match reply.best {
        None => {
            // B-3 — Black has no legal move: split checkmate vs stalemate by check.
            let status = if is_in_check(&overview.board, false) {
                GameStatus::BlackInCheckmate
            } else {
                GameStatus::BlackInStalemate
            };
            // Business Decision (rule 4) + Feature outcome: game over, no move applied.
            log_info_f!(LogFeature::PlayWithStockfish.as_str(), session, tracking, uuid = %id, status = status.as_str(), "engine reports no Black move (game over)");
            OpponentOutcome::Ok(Box::new(OpponentReply {
                from: None,
                to: None,
                piece: None,
                capture: None,
                status,
                overview,
            }))
        }
        Some(uci) => {
            let piece = overview.board[uci.from.row][uci.from.col];
            let (next_board, capture) = apply_black_move(&overview.board, &uci);
            let status = game_status(&next_board, reply.mate_in);

            // Invariant / Rule Check (rule 6): the resulting position is scored (B-6) —
            // white-in-checkmate from the engine's mate, else white-in-check / move.
            log_info_f!(LogFeature::PlayWithStockfish.as_str(), session, tracking, uuid = %id, from = %uci.from, to = %uci.to, status = status.as_str(), "game status scored");

            // Business Decision (rule 4): the reply path — a capture or a quiet move.
            let move_kind = if capture.is_some() { "capture" } else { "quiet" };
            log_info_f!(LogFeature::PlayWithStockfish.as_str(), session, tracking, uuid = %id, move_kind = move_kind, capture = ?capture, "reply path selected");

            // B-8 — apply the reply to the stored board and record any capture.
            let stored = {
                let mut guard = registry.lock().expect("registry mutex poisoned");
                let game = match guard.get_mut(&id) {
                    Some(game) => game,
                    None => return OpponentOutcome::UnknownGame,
                };
                game.overview.board = next_board;
                if let Some(taken) = capture {
                    game.taken.push(taken);
                }
                game.overview.clone()
            };
            // State Change (rule 5): the game's stored Overview is now updated.
            log_info_f!(LogFeature::PlayWithStockfish.as_str(), session, tracking, uuid = %id, from = %uci.from, to = %uci.to, "board updated with Black's reply");

            OpponentOutcome::Ok(Box::new(OpponentReply {
                from: Some(uci.from.algebraic()),
                to: Some(uci.to.algebraic()),
                piece,
                capture,
                status,
                overview: stored,
            }))
        }
    }
}

// ---- unit tests: to_fen, parse_uci, game_status, apply_black_move -----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::piece::{Kind, Piece};

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

    fn overview(rows: [&str; 8]) -> Overview {
        Overview { board: board(rows), white: "both".to_string(), black: "both".to_string() }
    }

    #[test]
    fn to_fen_serializes_standard_opening_black_to_move() {
        // The F0001 standard opening → the FEN placement, always Black to move,
        // with `-`/`-`/`0 1` for the fields the app does not track (B-4).
        let ov = overview([
            "r n b q k b n r",
            "p p p p p p p p",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            "P P P P P P P P",
            "R N B Q K B N R",
        ]);
        assert_eq!(
            to_fen(&ov),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b - - 0 1"
        );
    }

    #[test]
    fn to_fen_compresses_empty_runs() {
        // After 1.e4 — a mixed rank exercises the run-length digits (B-4).
        let ov = overview([
            "r n b q k b n r",
            "p p p p p p p p",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . P . . .",
            ". . . . . . . .",
            "P P P P . P P P",
            "R N B Q K B N R",
        ]);
        assert_eq!(
            to_fen(&ov),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b - - 0 1"
        );
    }

    #[test]
    fn parse_uci_plain_and_promotion() {
        // A plain move: two squares, no promotion (B-5).
        let m = parse_uci("c7c5").expect("valid uci");
        assert_eq!(m.from, Square::parse("c7").unwrap());
        assert_eq!(m.to, Square::parse("c5").unwrap());
        assert_eq!(m.promo, None);

        // A promotion move: the trailing letter is read as a Kind (B-5, B-6a).
        let p = parse_uci("a2a1q").expect("valid uci");
        assert_eq!(p.from, Square::parse("a2").unwrap());
        assert_eq!(p.to, Square::parse("a1").unwrap());
        assert_eq!(p.promo, Some(Kind::Queen));

        // Too short → None.
        assert_eq!(parse_uci("a1"), None);
    }

    #[test]
    fn apply_black_move_records_capture_and_promotion() {
        // Black pawn e5 takes a white pawn on d4 (B-8).
        let b = board([
            ". . . . k . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . P . . . .",
            ". . . . . . . .",
            ". . . . p . . .",
            ". . . . K . . .",
        ]);
        // A black pawn on e2 (row 6) capturing d1 would promote; use a simpler
        // capture: put a black rook on e8 taking a white pawn on e2.
        let capture_board = board([
            ". . . . r . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . P . . .",
            ". . . . K . . .",
        ]);
        let m = parse_uci("e8e2").unwrap();
        let (next, capture) = apply_black_move(&capture_board, &m);
        assert_eq!(capture, Some(Piece::WhitePawn), "the taken white pawn is recorded (B-8)");
        assert_eq!(next[Square::parse("e8").unwrap().row][Square::parse("e8").unwrap().col], None, "source emptied");
        assert_eq!(next[Square::parse("e2").unwrap().row][Square::parse("e2").unwrap().col], Some(Piece::BlackRook), "rook on target");
        let _ = b; // (the first board is illustrative context only)

        // Promotion: a black pawn on a2 pushes to a1 and becomes a queen (B-6a).
        let promo_board = board([
            ". . . . k . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            "p . . . . . . .",
            ". . . . K . . .",
        ]);
        let pm = parse_uci("a2a1q").unwrap();
        let (promoted, _) = apply_black_move(&promo_board, &pm);
        assert_eq!(promoted[Square::parse("a1").unwrap().row][Square::parse("a1").unwrap().col], Some(Piece::BlackQueen), "pawn promoted to a black queen (B-6a)");
    }

    #[test]
    fn game_status_scores_the_position_after_the_reply() {
        // A position where White's king is attacked (Black rook on the e-file).
        let checked = board([
            ". . . . r . k .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . K . . .",
        ]);
        // mate_in == Some(1) → white-in-checkmate, whatever is_in_check says (B-6).
        assert_eq!(game_status(&checked, Some(1)), GameStatus::WhiteInCheckmate);
        // else a `cp` score with White in check → white-in-check (server-decided).
        assert_eq!(game_status(&checked, None), GameStatus::WhiteInCheck);

        // A quiet position — White not in check → move.
        let quiet = board([
            ". . . . k . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . . . . .",
            ". . . . K . . .",
        ]);
        assert_eq!(game_status(&quiet, None), GameStatus::Move);
        // A far mate (mate in 2+) is not an immediate mate → scored by check only.
        assert_eq!(game_status(&quiet, Some(3)), GameStatus::Move);
    }
}
