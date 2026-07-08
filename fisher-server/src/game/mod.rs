//! Game lifecycle: the `GAME` model, the in-memory registry, and the
//! `start_game` business delegate (F0001 rule B-1).

pub mod overview;
pub mod piece;
pub mod square;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use overview::{Overview, random_overview, standard_overview};
use piece::Piece;

use crate::proof_log::LogFeature;

/// Requested layout mode (validated at the handler edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Standard,
    Random,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Standard => "standard",
            Mode::Random => "random",
        }
    }
}

/// A created game: its id and the position served to the front end.
#[derive(Debug, Clone)]
pub struct Game {
    pub uuid: Uuid,
    pub mode: Mode,
    pub overview: Overview,
    /// Black pieces captured by White over the game (rule F0002 B-7). Stays
    /// server-side; a captured-pieces panel is a later feature.
    pub taken: Vec<Piece>,
}

/// Thread-safe in-memory registry of games, keyed by `uuid`.
pub type Registry = Arc<Mutex<HashMap<Uuid, Game>>>;

/// Create a new game (fresh v4 `uuid`), build its `Overview` for the requested
/// `mode`/`pieces`, register it, and return `(uuid, overview)` (rule B-1).
///
/// `pieces` is ignored for `Mode::Standard`. `session`/`tracking` are the
/// `follower` ids threaded through for the proof log.
pub fn start_game(registry: &Registry, mode: Mode, pieces: u8, session: &str, tracking: &str) -> (Uuid, Overview) {
    let uuid = Uuid::new_v4();

    let overview = match mode {
        Mode::Standard => standard_overview(),
        Mode::Random => random_overview(pieces),
    };
    // Business Milestone (rule 7): the position was built.
    log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, uuid = %uuid, mode = mode.as_str(), "board generated");

    let game = Game { uuid, mode, overview: overview.clone(), taken: Vec::new() };

    registry
        .lock()
        .expect("registry mutex poisoned")
        .insert(uuid, game);

    // State Change (rule 5): the game is now registered.
    log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, uuid = %uuid, mode = mode.as_str(), "game created");
    (uuid, overview)
}

/// Overwrite an existing game's stored `Overview` with `overview` (F0002 test
/// seam, rules T-1–T-7): whole-board replace, `uuid`/`mode` left unchanged.
/// Returns the now-stored `Overview` (echoed back), or `None` when `uuid` names
/// no game — setup-board never mints a game (rule T-7).
pub fn setup_board(registry: &Registry, uuid: &str, overview: Overview, session: &str, tracking: &str) -> Option<Overview> {
    let id = Uuid::parse_str(uuid).ok()?;
    let mut guard = registry.lock().expect("registry mutex poisoned");
    let game = guard.get_mut(&id)?;

    // State Change (rule 5): the stored position is replaced wholesale (T-4).
    game.overview = overview.clone();
    log_info_f!(LogFeature::MoveAPiece.as_str(), session, tracking, uuid = %id, "board setup installed");
    Some(overview)
}
