//! Integration tests for feature F0002 — move a white piece (IT-F0002).
//!
//! Black-box, through the public API: each test drives the running `fisher-server`
//! over HTTP with reqwest. Every move scenario first seeds a known position via
//! the private `POST /private/setup-board` seam (rules T-1–T-7), then drives
//! `POST /move-a-piece`. No mock, no internal-state inspection — only HTTP
//! status, headers and JSON body are observed. Each assertion ties its outcome
//! back to the rule it proves (see `_ai/features/F0002-move-a-piece/IT-F0002.md`).

use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

// ---- local deser structs (the test only knows the public wire shape) --------

#[derive(Debug, Deserialize)]
struct Overview {
    board: Vec<Vec<String>>,
    white: String,
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

#[derive(Debug, Deserialize)]
struct ErrorBody {
    error: String,
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

/// Position #1 — standard opening.
fn board_1() -> Vec<Vec<String>> {
    grid([
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

/// Position #2 — capture (White P e4, Black p d5, kings e1/e8).
fn board_2() -> Vec<Vec<String>> {
    grid([
        ". . . . k . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . p . . . .",
        ". . . . P . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . K . . .",
    ])
}

/// Position #3 — pin (White K e1, White B e2, Black r e8, Black k a8).
fn board_3() -> Vec<Vec<String>> {
    grid([
        "k . . . r . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . B . . .",
        ". . . . K . . .",
    ])
}

/// Position #4 — historical middle game (Opera Game after White's 8.Nc3).
fn board_4() -> Vec<Vec<String>> {
    grid([
        "r n . . k b . r",
        "p p p . q p p p",
        ". . . . . n . .",
        ". . . . p . . .",
        ". . B . P . . .",
        ". Q N . . . . .",
        "P P P . . P P P",
        "R . B . K . . R",
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
/// assert the echo took (rule T-6), and return the game `uuid` ready to move against.
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

// ---- Seed seam — POST /private/setup-board ----------------------------------

// ---- TC-IT-F0002-001 --------------------------------------------------------

#[tokio::test]
async fn t10_f0002_setup_board_round_trip() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct StartGameResponse {
        uuid: String,
    }
    let start: StartGameResponse = client.get(format!("{base}/start-game")).send().await?.json().await?;
    let board = board_2();

    let resp = client
        .post(format!("{base}/private/setup-board"))
        .json(&json!({ "uuid": start.uuid, "board": board, "white": "both", "black": "both" }))
        .send()
        .await?;

    // 1. the seed succeeds for a known game (T-2).
    assert_eq!(resp.status(), 200, "setup-board succeeds for a known game (T-2)");
    let body: SetupBoardResponse = resp.json().await?;

    // 2. whole-board override took and is echoed (T-4, T-6).
    assert_eq!(body.overview.board, board, "echoed board equals the installed board (T-4, T-6)");
    assert_eq!(cell(&body.overview, "e4"), "P", "e4 present in the echoed board (T-6)");
    assert_eq!(cell(&body.overview, "d5"), "p", "d5 present in the echoed board (T-6)");

    // 3. defaults stored and echoed (T-6).
    assert_eq!(body.overview.white, "both", "white both echoed (T-6)");
    assert_eq!(body.overview.black, "both", "black both echoed (T-6)");
    Ok(())
}

// ---- TC-IT-F0002-002 --------------------------------------------------------

#[tokio::test]
async fn t20_f0002_setup_board_malformed_400() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct StartGameResponse {
        uuid: String,
    }
    let start: StartGameResponse = client.get(format!("{base}/start-game")).send().await?.json().await?;

    // A 7×8 grid (a row dropped) → not an 8×8 grid (T-2).
    let seven_by_eight: Vec<Vec<String>> = board_1().into_iter().take(7).collect();
    let resp = client
        .post(format!("{base}/private/setup-board"))
        .json(&json!({ "uuid": start.uuid, "board": seven_by_eight }))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "a 7×8 grid is rejected before touching game state (T-2)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "invalid board setup", "stable error for a bad grid (T-2)");

    // An 8×8 grid with a cell "X" → cell is not a valid piece letter (T-3).
    let mut bad_cell = board_1();
    bad_cell[3][3] = "X".to_string();
    let resp = client
        .post(format!("{base}/private/setup-board"))
        .json(&json!({ "uuid": start.uuid, "board": bad_cell }))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "an invalid cell is rejected (T-3)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "invalid board", "stable error for a bad cell (T-3)");
    Ok(())
}

// ---- TC-IT-F0002-003 --------------------------------------------------------

#[tokio::test]
async fn t30_f0002_setup_board_unknown_uuid_404() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();
    let unknown = Uuid::new_v4().to_string();

    let resp = client
        .post(format!("{base}/private/setup-board"))
        .json(&json!({ "uuid": unknown, "board": board_1() }))
        .send()
        .await?;

    // 1. setup-board only overwrites an existing game (T-7).
    assert_eq!(resp.status(), 404, "setup-board on an unknown game is 404 (T-7)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "unknown game", "stable unknown-game message (T-7)");

    // 3. a follow-up move on the same uuid is also 404 — nothing was minted (T-7).
    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": unknown, "from": "d2", "to": "d4" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 404, "no game was minted by setup-board (T-7)");
    Ok(())
}

// ---- Move route — POST /move-a-piece ----------------------------------------

// ---- TC-IT-F0002-004 --------------------------------------------------------

#[tokio::test]
async fn t40_f0002_valid_pawn_push() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_1()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "d2", "to": "d4" }))
        .send()
        .await?;

    // 1. the move is applied (B-8, B-12).
    assert_eq!(resp.status(), 200, "a valid pawn double push is applied (B-8, B-12)");
    let body: MoveResponse = resp.json().await?;

    // 2. moved piece echoed, a push takes nothing (B-6, B-12).
    assert_eq!(body.piece, "P", "the moved pawn is echoed (B-12)");
    assert_eq!(body.capture, None, "a push takes nothing (B-6)");

    // 3. the apply step moved the pawn (B-7).
    assert_eq!(cell(&body.overview, "d2"), "", "d2 emptied (B-7)");
    assert_eq!(cell(&body.overview, "d4"), "P", "d4 filled (B-7)");

    // 4. castling fields carried unchanged (B-7).
    assert_eq!(body.overview.white, "both", "white carried unchanged (B-7)");
    assert_eq!(body.overview.black, "both", "black carried unchanged (B-7)");
    Ok(())
}

// ---- TC-IT-F0002-005 --------------------------------------------------------

#[tokio::test]
async fn t50_f0002_valid_diagonal_capture() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_2()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "e4", "to": "d5" }))
        .send()
        .await?;

    // 1. the diagonal capture is legal and applied (B-12, B-6).
    assert_eq!(resp.status(), 200, "a valid diagonal capture is applied (B-12, B-6)");
    let body: MoveResponse = resp.json().await?;

    // 2. the taken black pawn is reported (B-6, B-8).
    assert_eq!(body.piece, "P", "the moved pawn is echoed (B-12)");
    assert_eq!(body.capture, Some("p".to_string()), "the taken black pawn is reported (B-6, B-8)");

    // 3. the pawn moved onto the target and overwrote the take (B-7, B-6).
    assert_eq!(cell(&body.overview, "e4"), "", "e4 emptied (B-7)");
    assert_eq!(cell(&body.overview, "d5"), "P", "d5 now holds the pawn, the take is gone (B-7, B-6)");
    Ok(())
}

// ---- TC-IT-F0002-006 --------------------------------------------------------

#[tokio::test]
async fn t60_f0002_not_your_piece() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // Empty source, then a black source — both "not your piece" (B-2).
    for (from, to) in [("e4", "e5"), ("d7", "d5")] {
        let uuid = seed(&base, board_1()).await?;
        let resp = client
            .post(format!("{base}/move-a-piece"))
            .json(&json!({ "uuid": uuid, "from": from, "to": to }))
            .send()
            .await?;
        assert_eq!(resp.status(), 422, "a non-white source is illegal, not malformed ({from}->{to}, B-2)");
        let body: IllegalBody = resp.json().await?;
        assert_eq!(body.reason, "not your piece", "precise reason for a non-white source ({from}->{to}, B-2)");
    }
    Ok(())
}

// ---- TC-IT-F0002-007 --------------------------------------------------------

#[tokio::test]
async fn t70_f0002_own_piece_on_target() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_1()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "g1", "to": "e2" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 422, "a self-capture is illegal (B-3)");
    let body: IllegalBody = resp.json().await?;
    assert_eq!(body.reason, "own piece on target", "precise reason: cannot take your own piece (B-3)");
    Ok(())
}

// ---- TC-IT-F0002-008 --------------------------------------------------------

#[tokio::test]
async fn t80_f0002_illegal_shape() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // Over-long push, then a non-capturing diagonal — both "illegal shape" (B-4, B-12).
    for (from, to) in [("d2", "d5"), ("e2", "d3")] {
        let uuid = seed(&base, board_1()).await?;
        let resp = client
            .post(format!("{base}/move-a-piece"))
            .json(&json!({ "uuid": uuid, "from": from, "to": to }))
            .send()
            .await?;
        assert_eq!(resp.status(), 422, "a shape that fits no geometry is illegal ({from}->{to}, B-4)");
        let body: IllegalBody = resp.json().await?;
        assert_eq!(body.reason, "illegal shape", "precise reason for a bad shape ({from}->{to}, B-4, B-12)");
    }
    Ok(())
}

// ---- TC-IT-F0002-009 --------------------------------------------------------

#[tokio::test]
async fn t90_f0002_path_blocked() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // A queen and a rook each blocked by their own pawn — "path blocked" (B-5).
    for (from, to) in [("d1", "d3"), ("a1", "a4")] {
        let uuid = seed(&base, board_1()).await?;
        let resp = client
            .post(format!("{base}/move-a-piece"))
            .json(&json!({ "uuid": uuid, "from": from, "to": to }))
            .send()
            .await?;
        assert_eq!(resp.status(), 422, "a slider stopped in its path is illegal ({from}->{to}, B-5)");
        let body: IllegalBody = resp.json().await?;
        assert_eq!(body.reason, "path blocked", "precise reason, distinct from illegal shape ({from}->{to}, B-5)");
    }
    Ok(())
}

// ---- TC-IT-F0002-010 --------------------------------------------------------

#[tokio::test]
async fn t100_f0002_king_safety_pin_and_escape() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // 1. the pinned bishop move leaves White's king in check (B-14, B-15).
    let uuid = seed(&base, board_3()).await?;
    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "e2", "to": "d3" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 422, "moving the pinned bishop is illegal (B-14, B-15)");
    let body: IllegalBody = resp.json().await?;
    assert_eq!(body.reason, "king in check", "precise reason: the move leaves the king attacked (B-14)");

    // 2. the king may step off the file to a safe square (B-14, B-7).
    let uuid = seed(&base, board_3()).await?;
    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "e1", "to": "f1" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "a king-safe escape is applied (B-14), not a blanket refusal");
    let body: MoveResponse = resp.json().await?;
    assert_eq!(body.piece, "K", "the king is echoed (B-14)");
    assert_eq!(body.capture, None, "the quiet king move takes nothing (B-6)");
    assert_eq!(cell(&body.overview, "e1"), "", "e1 emptied (B-7)");
    assert_eq!(cell(&body.overview, "f1"), "K", "f1 now holds the king (B-7)");
    Ok(())
}

// ---- TC-IT-F0002-011 --------------------------------------------------------

#[tokio::test]
async fn t110_f0002_malformed_request_400() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_1()).await?;
    let client = reqwest::Client::new();

    // from == to → invalid move request (B-1).
    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "d2", "to": "d2" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "from == to is a malformed request (B-1)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "invalid move request", "stable error for from == to (B-1)");

    // to out of range → invalid square (B-1).
    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "d2", "to": "d9" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "an out-of-range square is a malformed request (B-1)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "invalid square", "stable error for a bad square (B-1)");

    // missing `to` field → invalid move request (B-1).
    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "d2" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "a missing field is a malformed request (B-1)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "invalid move request", "stable error for a missing field (B-1)");
    Ok(())
}

// ---- TC-IT-F0002-012 --------------------------------------------------------

#[tokio::test]
async fn t120_f0002_unknown_game_404() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();
    let unknown = Uuid::new_v4().to_string();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": unknown, "from": "d2", "to": "d4" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 404, "a move on an unknown game is 404 (B-1)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "unknown game", "stable unknown-game message (B-1)");
    Ok(())
}

// ---- TC-IT-F0002-013 --------------------------------------------------------

#[tokio::test]
async fn t130_f0002_cors_allow_origin() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_1()).await?;
    let client = reqwest::Client::new();
    let origin = "http://localhost:5173";

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .header("Origin", origin)
        .json(&json!({ "uuid": uuid, "from": "d2", "to": "d4" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "the seeded move is applied under the CORS layer");
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("CORS allow-origin header present (B-10)")
        .to_str()?
        .to_string();
    assert!(acao == origin || acao == "*", "CORS permits the front dev origin (B-10), got {acao:?}");
    Ok(())
}

// ---- Valid piece-type moves — seeded from Position #4 (Opera Game) ----------

// ---- TC-IT-F0002-014 --------------------------------------------------------

#[tokio::test]
async fn t140_f0002_valid_knight_move() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_4()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "c3", "to": "d5" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "the knight move is applied (B-8, B-12)");
    let body: MoveResponse = resp.json().await?;
    assert_eq!(body.piece, "N", "the knight is echoed (B-12)");
    assert_eq!(body.capture, None, "a quiet move takes nothing (B-6)");
    assert_eq!(cell(&body.overview, "c3"), "", "c3 emptied (B-7)");
    assert_eq!(cell(&body.overview, "d5"), "N", "the knight jumped to d5 (B-7, B-12, B-4)");
    Ok(())
}

// ---- TC-IT-F0002-015 --------------------------------------------------------

#[tokio::test]
async fn t150_f0002_valid_king_move() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_4()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "e1", "to": "f1" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "a one-square king move to a safe square is applied (B-4, B-14)");
    let body: MoveResponse = resp.json().await?;
    assert_eq!(body.piece, "K", "the king is echoed (B-4)");
    assert_eq!(body.capture, None, "the quiet move takes nothing (B-6)");
    assert_eq!(cell(&body.overview, "e1"), "", "e1 emptied (B-7)");
    assert_eq!(cell(&body.overview, "f1"), "K", "the king moved to f1 (B-7)");
    assert_eq!(body.overview.white, "both", "castling fields carried unchanged (B-7)");
    Ok(())
}

// ---- TC-IT-F0002-016 --------------------------------------------------------

#[tokio::test]
async fn t160_f0002_valid_queen_capture() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_4()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "b3", "to": "b7" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "the queen's sliding capture over a clear file is applied (B-5, B-12, B-8)");
    let body: MoveResponse = resp.json().await?;
    assert_eq!(body.piece, "Q", "the queen is echoed (B-12)");
    assert_eq!(body.capture, Some("p".to_string()), "the taken black pawn is reported (B-6, B-8)");
    assert_eq!(cell(&body.overview, "b3"), "", "b3 emptied (B-7)");
    assert_eq!(cell(&body.overview, "b7"), "Q", "the queen took on b7, the pawn is gone (B-7, B-6)");
    Ok(())
}

// ---- TC-IT-F0002-017 --------------------------------------------------------

#[tokio::test]
async fn t170_f0002_valid_rook_move() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_4()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "a1", "to": "b1" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "the rook's move along a clear rank is applied (B-5, B-12, B-8)");
    let body: MoveResponse = resp.json().await?;
    assert_eq!(body.piece, "R", "the rook is echoed (B-12)");
    assert_eq!(body.capture, None, "a quiet move takes nothing (B-6)");
    assert_eq!(cell(&body.overview, "a1"), "", "a1 emptied (B-7)");
    assert_eq!(cell(&body.overview, "b1"), "R", "the rook moved to b1 (B-7)");
    assert_eq!(body.overview.white, "both", "castling fields carried unchanged (B-7)");
    Ok(())
}

// ---- TC-IT-F0002-018 --------------------------------------------------------

#[tokio::test]
async fn t180_f0002_valid_bishop_capture() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_4()).await?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "c4", "to": "f7" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "the bishop's diagonal capture over a clear path is applied (B-5, B-12, B-8)");
    let body: MoveResponse = resp.json().await?;
    assert_eq!(body.piece, "B", "the bishop is echoed (B-12)");
    assert_eq!(body.capture, Some("p".to_string()), "the taken black pawn is reported (B-6, B-8)");
    assert_eq!(cell(&body.overview, "c4"), "", "c4 emptied (B-7)");
    assert_eq!(cell(&body.overview, "f7"), "B", "the bishop took on f7, the pawn is gone (B-7, B-6)");
    Ok(())
}
