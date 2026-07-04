//! fisher-server library: the Axum application that serves `GET /start-game`
//! (feature F0001). Exposed as a library so integration tests can boot the
//! real app on an ephemeral port.

#[macro_use]
pub mod proof_log;
pub mod game;

use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use rand::Rng;
use serde::Serialize;
use tower_http::cors::CorsLayer;

use game::{Mode, Registry, overview::Overview, start_game};
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
        .allow_methods([Method::GET]);

    Router::new()
        .route("/start-game", get(start_game_handler))
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
