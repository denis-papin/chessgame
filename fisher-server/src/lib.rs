//! fisher-server library: the Axum application that serves `GET /start-game`
//! (feature F0001). Exposed as a library so integration tests can boot the
//! real app on an ephemeral port.

pub mod game;

use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;

use game::{Mode, Registry, overview::Overview, start_game};

/// The front dev origin allowed by CORS (rule B-9 / architecture.md §6).
pub const FRONT_DEV_ORIGIN: &str = "http://localhost:5173";

/// Default piece count for `random` when `pieces` is omitted (rule B-3).
const DEFAULT_PIECES: u8 = 16;
const MIN_PIECES: u8 = 2;
const MAX_PIECES: u8 = 16;

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

fn bad_request(message: &str) -> Response {
    tracing::warn!(error = message, "request rejected");
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
    tracing::info!("start-game requested");

    // mode: default "standard"; case-sensitive match (Inputs / Errors).
    let mode = match params.get("mode").map(String::as_str) {
        None | Some("standard") => Mode::Standard,
        Some("random") => Mode::Random,
        Some(_) => return bad_request("invalid mode"),
    };

    // pieces: only meaningful for random; default 16; integer in 2..=16.
    let pieces = if mode == Mode::Random {
        match params.get("pieces") {
            None => DEFAULT_PIECES,
            Some(raw) => match raw.parse::<i64>() {
                Ok(n) if (MIN_PIECES as i64..=MAX_PIECES as i64).contains(&n) => n as u8,
                _ => return bad_request("invalid piece count"),
            },
        }
    } else {
        DEFAULT_PIECES // ignored for standard
    };

    let (uuid, overview) = start_game(&state.registry, mode, pieces);

    let body = StartGameResponse {
        uuid: uuid.to_string(),
        mode: mode.as_str().to_string(),
        overview,
    };
    tracing::info!(uuid = %uuid, "start-game served");
    (StatusCode::OK, Json(body)).into_response()
}
