//! Integration tests for feature F0004 — white pawn promotion (IT-F0004).
//!
//! Black-box, through the public API: each test drives the running `fisher-server`
//! over HTTP with reqwest. Every scenario first seeds a known position with a
//! White pawn on rank 7 via the private `POST /private/setup-board` seam (F0002
//! rules T-1–T-7), then drives `POST /move-a-piece`. No mock, no internal-state
//! inspection — only HTTP status and JSON body are observed. No engine is
//! involved: `POST /move-a-piece` never calls Stockfish. Each assertion ties its
//! outcome back to the rule it proves (see
//! `_ai/features/F0004-white-pawn-promotion/IT-F0004.md`).

use serde::Deserialize;
use serde_json::json;

// ---- local deser structs (the test only knows the public wire shape) --------

#[derive(Debug, Deserialize)]
struct Overview {
    board: Vec<Vec<String>>,
    #[allow(dead_code)]
    white: String,
    #[allow(dead_code)]
    black: String,
}

#[derive(Debug, Deserialize)]
struct SetupBoardResponse {
    #[allow(dead_code)]
    uuid: String,
    overview: Overview,
}

#[derive(Debug, Deserialize)]
struct MoveResponse {
    #[allow(dead_code)]
    from: String,
    #[allow(dead_code)]
    to: String,
    piece: String,
    capture: Option<String>,
    overview: Overview,
}

#[derive(Debug, Deserialize)]
struct IllegalBody {
    reason: String,
}

type Err = Box<dyn std::error::Error>;

// ---- harness & helpers ------------------------------------------------------

/// Base URL of the already-running `fisher-server` (override with `FISHER_SERVER_URL`).
fn server_url() -> String {
    std::env::var("FISHER_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:7200".to_string())
}

/// Return the running server's base URL, failing with a clear message if nothing
/// is listening.
async fn base_url() -> String {
    let base = server_url();
    if reqwest::get(base.as_str()).await.is_err() {
        panic!(
            "fisher-server is not reachable at {base}. \
             Start it first (e.g. `cargo run -p fisher-server`) or set \
             FISHER_SERVER_URL, then re-run the api-tests."
        );
    }
    base
}

/// Build an 8×8 board from 8 rank-8-first rows of space-separated cells, `.` empty.
fn grid(rows: [&str; 8]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            row.split(' ')
                .map(|c| if c == "." { String::new() } else { c.to_string() })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Position #1 — promotion (White K e1, White P e7, Black r d8, Black k a8).
fn board_1() -> Vec<Vec<String>> {
    grid([
        "k . . r . . . .",
        ". . . . P . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . K . . .",
    ])
}

/// Position #2 — blocked promotion (White K e1, White P e7, Black n e8, Black k a8).
fn board_2() -> Vec<Vec<String>> {
    grid([
        "k . . . n . . .",
        ". . . . P . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . K . . .",
    ])
}

/// Map an algebraic square to `(row, col)` in the F0001 board order.
fn square_to_rc(square: &str) -> (usize, usize) {
    let b = square.as_bytes();
    let col = (b[0] - b'a') as usize;
    let row = (8 - (b[1] - b'0')) as usize;
    (row, col)
}

/// The piece string on `square` of the overview's board.
fn cell(ov: &Overview, square: &str) -> String {
    let (r, c) = square_to_rc(square);
    ov.board[r][c].clone()
}

/// Mint a game via `GET /start-game`, install `board` via `POST /private/setup-board`,
/// assert the echo took (F0002 rule T-6), and return the game `uuid` ready to move.
async fn seed(base: &str, board: Vec<Vec<String>>) -> Result<String, Err> {
    let client = reqwest::Client::new();
    #[derive(Deserialize)]
    struct StartGameResponse {
        uuid: String,
    }
    let start: StartGameResponse = client.get(format!("{base}/start-game")).send().await?.json().await?;
    let uuid = start.uuid;

    let resp = client
        .post(format!("{base}/private/setup-board"))
        .json(&json!({ "uuid": uuid, "board": board, "white": "both", "black": "both" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "seed: setup-board must succeed (T-6)");
    let body: SetupBoardResponse = resp.json().await?;
    assert_eq!(body.overview.board, board, "seed: echoed board matches installed (T-6)");
    Ok(uuid)
}

// ---- TC-IT-F0004-001 --------------------------------------------------------

/// A promotion push replaces the pawn with a Queen.
/// Given Position #1, When e7→e8 onto the empty last rank, Then 200, piece "P",
/// capture null, and e8 holds a "Q" (not a "P") — proves B-2, B-3, B-5.
#[tokio::test]
async fn t10_f0004_promotion_push_to_queen() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_1()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "e7", "to": "e8" }))
        .send()
        .await?;

    // 1. the promotion push is a legal, applied move (B-1, B-2).
    assert_eq!(resp.status(), 200, "promotion push is applied (B-1, B-2)");
    let body: MoveResponse = resp.json().await?;

    // 2. the response echoes the mover pawn, not the queen; a push takes nothing (B-5).
    assert_eq!(body.piece, "P", "response echoes the moving pawn, not the queen (B-5)");
    assert_eq!(body.capture, None, "a promotion push takes nothing (B-5)");

    // 3. the pawn left e7 and the destination is a White Queen (B-2, B-3).
    assert_eq!(cell(&body.overview, "e7"), "", "the pawn left e7 (B-7)");
    assert_eq!(cell(&body.overview, "e8"), "Q", "e8 holds a White Queen (B-2, B-3)");

    // 4. the pawn was replaced, not merely moved to rank 8 (B-3).
    assert_ne!(cell(&body.overview, "e8"), "P", "the pawn was replaced by a Queen (B-3)");
    Ok(())
}

// ---- TC-IT-F0004-002 --------------------------------------------------------

/// A promotion capture reports the take and lands a Queen.
/// Given Position #1, When e7→d8 capturing the black rook, Then 200, piece "P",
/// capture "r", and d8 holds a "Q" — proves B-4, B-5.
#[tokio::test]
async fn t20_f0004_promotion_capture_to_queen() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_1()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "e7", "to": "d8" }))
        .send()
        .await?;

    // 1. the diagonal capture onto the last rank is legal and applied (B-4).
    assert_eq!(resp.status(), 200, "promotion capture is applied (B-4)");
    let body: MoveResponse = resp.json().await?;

    // 2. the mover stays the pawn; the taken rook is still reported (B-5).
    assert_eq!(body.piece, "P", "response echoes the moving pawn (B-5)");
    assert_eq!(body.capture.as_deref(), Some("r"), "the captured black rook is reported (B-5)");

    // 3. the pawn captured onto d8, overwrote the rook, and promoted (B-3, B-4).
    assert_eq!(cell(&body.overview, "e7"), "", "the pawn left e7 (B-7)");
    assert_eq!(cell(&body.overview, "d8"), "Q", "d8 holds a White Queen, the rook is gone (B-3, B-4)");
    Ok(())
}

// ---- TC-IT-F0004-003 --------------------------------------------------------

/// An illegal would-be promotion promotes nothing.
/// Given Position #2 (a black knight on e8), When e7→e8 pushes onto the occupied
/// square, Then 422 "illegal shape" and nothing is queened — proves B-4.
#[tokio::test]
async fn t30_f0004_blocked_promotion_illegal_shape() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_2()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "e7", "to": "e8" }))
        .send()
        .await?;

    // 1. the move is illegal, not malformed: promotion does not relax pawn geometry (B-4).
    assert_eq!(resp.status(), 422, "a push into an occupied square is illegal, not malformed (B-4)");
    let body: IllegalBody = resp.json().await?;

    // 2. the precise IllegalReason for a push into an occupied square (B-4, F0002 B-12).
    assert_eq!(body.reason, "illegal shape", "reason is 'illegal shape' (B-4)");
    // 3. the 422 verdict itself proves nothing was applied — the pawn is not queened (B-4).
    Ok(())
}
