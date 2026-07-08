//! fisher-server library: the Axum application that serves `GET /start-game`
//! (feature F0001). Exposed as a library so integration tests can boot the
//! real app on an ephemeral port.

#[macro_use]
pub mod proof_log;
pub mod game;
pub mod moves;

use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use game::piece::{self, Piece};
use game::square::Square;
use game::{Mode, Registry, overview::Overview, setup_board, start_game};
use moves::{MoveOutcome, move_piece};
use proof_log::LogFeature;

/// The front dev origin allowed by CORS (rule B-9 / architecture.md §6).
pub const FRONT_DEV_ORIGIN: &str = "http://localhost:5173";

/// Default piece count for `random` when `pieces` is omitted (rule B-3).
const DEFAULT_PIECES: u8 = 16;
const MIN_PIECES: u8 = 2;
const MAX_PIECES: u8 = 16;

/// Placeholder session id for the `follower` trailer (proof-logs.md). A fixed
/// constant for now, until the front-to-back session propagation is wired up.
const SESSION_ID: &str = "1234";

/// Lower/upper bounds of the placeholder request tracking id (inclusive).
const TRACKING_MIN: u32 = 10_000;
const TRACKING_MAX: u32 = 99_999;

/// Produce the `follower` ids for one request: the fixed placeholder `session`
/// and a fresh random `tracking` number in `10000..=99999`. Sourced once at the
/// handler edge and threaded through every proof log of the request. End-to-end
/// propagation of these ids across services is still being wired up
/// (proof-logs.md), so these placeholders stand in for now.
fn follower_ids() -> (String, String) {
    let tracking = rand::thread_rng().gen_range(TRACKING_MIN..=TRACKING_MAX);
    (SESSION_ID.to_string(), tracking.to_string())
}

/// Shared application state: the in-memory game registry.
#[derive(Clone, Default)]
pub struct AppState {
    pub registry: Registry,
}

/// Build the application router with CORS for the front dev origin.
pub fn app() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            FRONT_DEV_ORIGIN
                .parse::<HeaderValue>()
                .expect("valid origin"),
        )
        // F0002 rule B-10 adds the POST method for `/move-a-piece`.
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    Router::new()
        .route("/start-game", get(start_game_handler))
        .route("/move-a-piece", post(move_piece_handler))
        .route("/private/setup-board", post(setup_board_handler))
        .layer(cors)
        .with_state(AppState::default())
}

/// Serve the application on an already-bound listener (used by main and tests).
pub async fn serve(listener: tokio::net::TcpListener) {
    axum::serve(listener, app()).await.expect("server error");
}

/// Success body of `GET /start-game`.
#[derive(Serialize)]
struct StartGameResponse {
    uuid: String,
    mode: String,
    overview: Overview,
}

/// Error body for rejected requests.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn bad_request(message: &str, session: &str, tracking: &str) -> Response {
    // Error / Exception (rule 8) and Feature Exit (rule 2) on the failure path:
    // this terminal log carries the 🏁 exit marker and records result=FAILURE.
    log_warn_f!(LogFeature::StartAGame.as_str(), session, tracking, result = "FAILURE", error = message, "🏁 request rejected");
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody { error: message.to_string() }),
    )
        .into_response()
}

/// `GET /start-game` — validate `mode` and (for `random`) `pieces`, delegate to
/// `game::start_game`, and return `{ uuid, mode, overview }`.
async fn start_game_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let (session, tracking) = follower_ids();
    // Feature Entry (rule 1): carries the 🚀 boundary marker.
    log_info_f!(LogFeature::StartAGame.as_str(), &session, &tracking, "🚀 start-game requested");

    // mode: default "standard"; case-sensitive match (Inputs / Errors).
    let mode = match params.get("mode").map(String::as_str) {
        None | Some("standard") => Mode::Standard,
        Some("random") => Mode::Random,
        Some(_) => return bad_request("invalid mode", &session, &tracking),
    };
    // Business Decision (rule 4): the layout path is now chosen.
    log_info_f!(LogFeature::StartAGame.as_str(), &session, &tracking, mode = mode.as_str(), "layout mode selected");

    // pieces: only meaningful for random; default 16; integer in 2..=16.
    let pieces = if mode == Mode::Random {
        match params.get("pieces") {
            None => DEFAULT_PIECES,
            Some(raw) => match raw.parse::<i64>() {
                Ok(n) if (MIN_PIECES as i64..=MAX_PIECES as i64).contains(&n) => n as u8,
                _ => return bad_request("invalid piece count", &session, &tracking),
            },
        }
    } else {
        DEFAULT_PIECES // ignored for standard
    };
    // Invariant / Rule Check (rule 6): the piece-count rule was evaluated and
    // passed. Only random carries a count; standard ignores it.
    if mode == Mode::Random {
        log_info_f!(LogFeature::StartAGame.as_str(), &session, &tracking, pieces = pieces, piece_count_valid = true, "piece count validated");
    }

    let (uuid, overview) = start_game(&state.registry, mode, pieces, &session, &tracking);

    let body = StartGameResponse {
        uuid: uuid.to_string(),
        mode: mode.as_str().to_string(),
        overview,
    };
    // Feature Exit (rule 2): the routine completed successfully; 🏁 boundary marker.
    log_info_f!(LogFeature::StartAGame.as_str(), &session, &tracking, uuid = %uuid, result = "SUCCESS", "🏁 start-game served");
    (StatusCode::OK, Json(body)).into_response()
}

// ---- F0002 — move a piece (POST /move-a-piece) ------------------------------

/// Move request body (rule B-1). All fields optional so a missing one is a `400`
/// business rejection rather than an Axum deserialize error.
#[derive(Deserialize)]
struct MoveRequest {
    uuid: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

/// Success body of `POST /move-a-piece` (rule B-8): the applied move + board.
#[derive(Serialize)]
struct MoveResponse {
    from: String,
    to: String,
    piece: Piece,
    capture: Option<Piece>,
    overview: Overview,
}

/// Illegal-move body (`422`, rule B-8): the closed-set `reason`.
#[derive(Serialize)]
struct IllegalBody {
    reason: String,
}

/// `400 Bad Request` on the move route — malformed request, not an illegal move.
/// Carries the failure Feature Exit (🏁) for the proof line.
fn move_bad_request(message: &str, session: &str, tracking: &str) -> Response {
    log_warn_f!(LogFeature::MoveAPiece.as_str(), session, tracking, result = "FAILURE", error = message, "🏁 move request rejected");
    (StatusCode::BAD_REQUEST, Json(ErrorBody { error: message.to_string() })).into_response()
}

/// `POST /move-a-piece` — validate the request body, delegate to `moves::move_piece`,
/// and map the outcome to the HTTP status: `200` applied, `422` illegal, `400`
/// malformed, `404` unknown game (rules B-1, B-8).
async fn move_piece_handler(State(state): State<AppState>, Json(req): Json<MoveRequest>) -> Response {
    let (session, tracking) = follower_ids();
    // Feature Entry (rule 1): 🚀 boundary marker.
    log_info_f!(LogFeature::MoveAPiece.as_str(), &session, &tracking, "🚀 move-a-piece requested");

    // B-1 — required fields present.
    let (uuid, from, to) = match (req.uuid, req.from, req.to) {
        (Some(u), Some(f), Some(t)) => (u, f, t),
        _ => return move_bad_request("invalid move request", &session, &tracking),
    };
    // B-1 — source and target must differ.
    if from == to {
        return move_bad_request("invalid move request", &session, &tracking);
    }
    // B-1 — both squares in a1..h8; parse into the typed coordinate the geometry
    // routines manipulate (the wire keeps the algebraic `from`/`to` strings).
    let (from_sq, to_sq) = match (Square::parse(&from), Square::parse(&to)) {
        (Some(f), Some(t)) => (f, t),
        _ => return move_bad_request("invalid square", &session, &tracking),
    };
    // Invariant / Rule Check (rule 6): the request contract (B-1) holds — uuid
    // present, from/to are squares in a1..h8, and from != to.
    log_info_f!(LogFeature::MoveAPiece.as_str(), &session, &tracking, uuid = %uuid, from = %from, to = %to, request_valid = true, "request contract validated");

    match move_piece(&state.registry, &uuid, from_sq, to_sq, &session, &tracking) {
        MoveOutcome::UnknownGame => {
            // Error / Exception (rule 8) + Feature Exit (rule 2): 🏁, result=FAILURE.
            log_warn_f!(LogFeature::MoveAPiece.as_str(), &session, &tracking, uuid = %uuid, result = "FAILURE", error = "unknown game", "🏁 move rejected: unknown game");
            (StatusCode::NOT_FOUND, Json(ErrorBody { error: "unknown game".to_string() })).into_response()
        }
        MoveOutcome::Illegal(reason) => {
            // Feature Exit (rule 2) on the illegal branch: 🏁, result=FAILURE.
            log_info_f!(LogFeature::MoveAPiece.as_str(), &session, &tracking, uuid = %uuid, from = %from, to = %to, result = "FAILURE", reason = reason.as_str(), "🏁 move refused (illegal)");
            (StatusCode::UNPROCESSABLE_ENTITY, Json(IllegalBody { reason: reason.as_str().to_string() })).into_response()
        }
        MoveOutcome::Applied { piece, capture, overview } => {
            // Feature Exit (rule 2): 🏁, result=SUCCESS.
            log_info_f!(LogFeature::MoveAPiece.as_str(), &session, &tracking, uuid = %uuid, from = %from, to = %to, result = "SUCCESS", "🏁 move applied");
            (StatusCode::OK, Json(MoveResponse { from, to, piece, capture, overview })).into_response()
        }
    }
}

// ---- F0002 test seam — setup a known board (POST /private/setup-board) -------

/// setup-board request (rules T-2, T-3). `board` optional so a missing one is a
/// `400` business rejection; `white`/`black` default to `"both"` (rule T-4).
#[derive(Deserialize)]
struct SetupBoardRequest {
    uuid: Option<String>,
    board: Option<Vec<Vec<String>>>,
    white: Option<String>,
    black: Option<String>,
}

/// setup-board success body (rule T-6): the now-stored `Overview`, echoed back.
#[derive(Serialize)]
struct SetupBoardResponse {
    uuid: String,
    overview: Overview,
}

/// `true` when `cell` is `""` or a single valid piece letter (rule T-3).
fn is_valid_cell(cell: &str) -> bool {
    piece::is_valid_cell(cell)
}

/// `true` when `value` is a valid `Castle` availability (rule T-2).
fn is_valid_castle(value: &str) -> bool {
    matches!(value, "both" | "short castle" | "long castle" | "none")
}

/// `400 Bad Request` on the setup-board route with a stable `error` message.
fn setup_bad_request(message: &str, session: &str, tracking: &str) -> Response {
    log_warn_f!(LogFeature::MoveAPiece.as_str(), session, tracking, result = "FAILURE", error = message, "🏁 setup-board rejected");
    (StatusCode::BAD_REQUEST, Json(ErrorBody { error: message.to_string() })).into_response()
}

/// `POST /private/setup-board` — private test seam (rules T-1–T-7): validate the
/// board, then overwrite the game's stored `Overview` and echo it back. No CORS,
/// no legality check, never mints a game.
async fn setup_board_handler(State(state): State<AppState>, Json(req): Json<SetupBoardRequest>) -> Response {
    let (session, tracking) = follower_ids();
    // Feature Entry (rule 1): 🚀 boundary marker.
    log_info_f!(LogFeature::MoveAPiece.as_str(), &session, &tracking, "🚀 setup-board requested");

    // T-2 — uuid present.
    let uuid = match req.uuid {
        Some(u) => u,
        None => return setup_bad_request("invalid board setup", &session, &tracking),
    };
    // T-2 — board present and an 8×8 grid.
    let board = match req.board {
        Some(b) if b.len() == 8 && b.iter().all(|row| row.len() == 8) => b,
        _ => return setup_bad_request("invalid board setup", &session, &tracking),
    };
    // T-3 — every cell is "" or a valid piece letter.
    if board.iter().flatten().any(|cell| !is_valid_cell(cell)) {
        return setup_bad_request("invalid board", &session, &tracking);
    }
    // T-2/T-4 — castling values valid; default to "both".
    let white = req.white.unwrap_or_else(|| "both".to_string());
    let black = req.black.unwrap_or_else(|| "both".to_string());
    if !is_valid_castle(&white) || !is_valid_castle(&black) {
        return setup_bad_request("invalid board setup", &session, &tracking);
    }

    // Cells validated above (T-3): parse the wire strings into the typed board.
    let board = board
        .iter()
        .map(|row| row.iter().map(|cell| Piece::cell_from_str(cell)).collect())
        .collect();

    let overview = Overview { board, white, black };
    match setup_board(&state.registry, &uuid, overview, &session, &tracking) {
        Some(stored) => {
            log_info_f!(LogFeature::MoveAPiece.as_str(), &session, &tracking, uuid = %uuid, result = "SUCCESS", "🏁 setup-board installed");
            (StatusCode::OK, Json(SetupBoardResponse { uuid, overview: stored })).into_response()
        }
        None => {
            // T-7 — unknown game is 404 and mints nothing.
            log_warn_f!(LogFeature::MoveAPiece.as_str(), &session, &tracking, result = "FAILURE", error = "unknown game", "🏁 setup-board rejected: unknown game");
            (StatusCode::NOT_FOUND, Json(ErrorBody { error: "unknown game".to_string() })).into_response()
        }
    }
}
