//! Game lifecycle: the `GAME` model, the in-memory registry, and the
//! `start_game` business delegate (F0001 rule B-1).

pub mod overview;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use overview::{Overview, random_overview, standard_overview};

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
}

/// Thread-safe in-memory registry of games, keyed by `uuid`.
pub type Registry = Arc<Mutex<HashMap<Uuid, Game>>>;

/// Create a new game (fresh v4 `uuid`), build its `Overview` for the requested
/// `mode`/`pieces`, register it, and return `(uuid, overview)` (rule B-1).
///
/// `pieces` is ignored for `Mode::Standard`.
pub fn start_game(registry: &Registry, mode: Mode, pieces: u8) -> (Uuid, Overview) {
    let overview = match mode {
        Mode::Standard => standard_overview(),
        Mode::Random => random_overview(pieces),
    };

    let uuid = Uuid::new_v4();
    let game = Game { uuid, mode, overview: overview.clone() };

    registry
        .lock()
        .expect("registry mutex poisoned")
        .insert(uuid, game);

    tracing::info!(uuid = %uuid, mode = mode.as_str(), "game created");
    (uuid, overview)
}
