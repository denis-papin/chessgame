//! Integration tests for feature F0003 — play with Stockfish (IT-F0003).
//!
//! Black-box, through the public API: each test drives the running `fisher-server`
//! over HTTP with reqwest, and the server in turn calls the **live** Stockfish
//! REST engine on `:4000`. Positions are seeded through the private
//! `POST /private/setup-board` seam (F0002 rules T-1–T-7), then `POST /opponent-move`
//! is driven. No mock, no internal-state inspection — only HTTP status, headers
//! and JSON body are observed. Each assertion ties its outcome back to the rule
//! it proves (see `_ai/features/F0003-play-with-stockfish/IT-F0003.md`).
//!
//! These tests require BOTH a running `fisher-server` AND a reachable Stockfish
//! container on `:4000` (the server's `ENGINE_BASE`). With the engine up, the
//! engine-driven cases pass; TC-009 (engine unavailable) is `#[ignore]`d because
//! it needs the container stopped, which the harness cannot do here (IT-F0003).

use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

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

/// The reply body of `POST /opponent-move` (F0003 Output). `from`/`to`/`piece`/
/// `capture` are `null` on a game-over result.
#[derive(Debug, Deserialize)]
struct OpponentMoveResponse {
    from: Option<String>,
    to: Option<String>,
    piece: Option<String>,
    capture: Option<String>,
    status: String,
    overview: Overview,
}

/// The success body of a White move (F0002) — used only to set up TC-010.
#[derive(Debug, Deserialize)]
struct MoveResponse {
    #[allow(dead_code)]
    piece: String,
    overview: Overview,
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
/// is listening. (The engine must also be up for the reply cases to succeed.)
async fn base_url() -> String {
    let base = server_url();
    if reqwest::get(base.as_str()).await.is_err() {
        panic!(
            "fisher-server is not reachable at {base}. Start it first \
             (e.g. `cargo run -p fisher-server`), ensure the Stockfish container \
             is up on :4000, or set FISHER_SERVER_URL, then re-run the api-tests."
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

/// Position #1 — capture (White Q d5 hanging to the Black e6 pawn).
fn board_1() -> Vec<Vec<String>> {
    grid([
        ". . . . k . . .",
        ". . . . . . . .",
        ". . . . p . . .",
        ". . . Q . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . K . . .",
    ])
}

/// Position #2 — Black checks White (knight fork Nf3+ on Kg1 / Qd2).
fn board_2() -> Vec<Vec<String>> {
    grid([
        ". . . . . . k .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . n .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . Q . . . .",
        ". . . . . . K .",
    ])
}

/// Position #3 — Black mates White (fool's-mate position; Qd8-h4#).
fn board_3() -> Vec<Vec<String>> {
    grid([
        "r n b q k b n r",
        "p p p p . p p p",
        ". . . . . . . .",
        ". . . . p . . .",
        ". . . . . . P .",
        ". . . . . P . .",
        "P P P P P . . P",
        "R N B Q K B N R",
    ])
}

/// Position #4 — Black is checkmated (Scholar's mate; White Q on f7 mates).
fn board_4() -> Vec<Vec<String>> {
    grid([
        "r . b q k b . r",
        "p p p p . Q p p",
        ". . n . . n . .",
        ". . . . p . . .",
        ". . B . P . . .",
        ". . . . . . . .",
        "P P P P . P P P",
        "R N B . K . N R",
    ])
}

/// Position #5 — Black is stalemated (White K g6, Q f7 box in Black K h8).
fn board_5() -> Vec<Vec<String>> {
    grid([
        ". . . . . . . k",
        ". . . . . Q . .",
        ". . . . . . K .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
        ". . . . . . . .",
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

/// Mint a game via `GET /start-game` and return its `uuid`.
async fn start_game(base: &str) -> Result<String, Err> {
    #[derive(Deserialize)]
    struct StartGameResponse {
        uuid: String,
    }
    let start: StartGameResponse = reqwest::Client::new()
        .get(format!("{base}/start-game"))
        .send()
        .await?
        .json()
        .await?;
    Ok(start.uuid)
}

/// Mint a game, install `board` via `POST /private/setup-board`, assert the echo
/// took, and return the game `uuid` ready to answer against (as in IT-F0002).
async fn seed(base: &str, board: Vec<Vec<String>>) -> Result<String, Err> {
    let client = reqwest::Client::new();
    let uuid = start_game(base).await?;

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

/// Issue `POST /opponent-move { uuid }` and return the raw response.
async fn opponent_move(base: &str, uuid: &str) -> Result<reqwest::Response, Err> {
    Ok(reqwest::Client::new()
        .post(format!("{base}/opponent-move"))
        .json(&json!({ "uuid": uuid }))
        .send()
        .await?)
}

// ---- TC-IT-F0003-001 --------------------------------------------------------

#[tokio::test]
async fn t10_f0003_opening_reply_is_a_move() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = start_game(&base).await?;

    let resp = opponent_move(&base, &uuid).await?;
    // 1. the server serialized a FEN, called the engine, and got a reply (B-4, B-5).
    assert_eq!(resp.status(), 200, "an opening game gets a reply (B-4, B-5)");
    let body: OpponentMoveResponse = resp.json().await?;

    // 2. an opening reply is neither check nor mate (B-6).
    assert_eq!(body.status, "move", "an opening reply is an ordinary move (B-6)");
    // 3. a legal black move was parsed and echoed (B-5).
    let from = body.from.as_deref().expect("from present for a move (B-5)");
    let to = body.to.as_deref().expect("to present for a move (B-5)");
    let piece = body.piece.as_deref().expect("piece present for a move (B-5)");
    assert_eq!(piece, piece.to_lowercase(), "the moved piece is a black (lowercase) piece (B-5)");
    // 4. an opening reply takes nothing (B-8).
    assert_eq!(body.capture, None, "an opening reply captures nothing (B-8)");
    // 5. the reply was applied to the stored board (B-8).
    assert_eq!(cell(&body.overview, from), "", "the source square is emptied (B-8)");
    assert_eq!(cell(&body.overview, to), piece, "the target holds the moved piece (B-8)");
    Ok(())
}

// ---- TC-IT-F0003-002 --------------------------------------------------------

#[tokio::test]
async fn t20_f0003_capture_reported_and_removed() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_1()).await?;

    let resp = opponent_move(&base, &uuid).await?;
    assert_eq!(resp.status(), 200, "the capturing reply is applied (B-6)");
    let body: OpponentMoveResponse = resp.json().await?;

    // 1. the capturing reply does not check the White king (B-6).
    assert_eq!(body.status, "move", "the capture is an ordinary move here (B-6)");
    // 2. the taken white queen is reported (B-8).
    assert_eq!(body.capture.as_deref(), Some("Q"), "the taken white queen is reported (B-8)");
    // 3. the black pawn lands on the queen's square, which is now vacated of White (B-8).
    let to = body.to.as_deref().expect("to present (B-5)");
    let from = body.from.as_deref().expect("from present (B-5)");
    assert_eq!(body.piece.as_deref(), Some("p"), "a black pawn moved (B-5)");
    assert_eq!(cell(&body.overview, to), "p", "the pawn overwrote the taken queen (B-8)");
    assert_eq!(cell(&body.overview, from), "", "the pawn's source is emptied (B-8)");
    Ok(())
}

// ---- TC-IT-F0003-003 --------------------------------------------------------

#[tokio::test]
async fn t30_f0003_reply_checks_white() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_2()).await?;

    let resp = opponent_move(&base, &uuid).await?;
    assert_eq!(resp.status(), 200, "the checking reply is applied (B-6)");
    let body: OpponentMoveResponse = resp.json().await?;

    // 1. the server flags a reply that leaves the White king attacked (B-6).
    // This status is computed by fisher-server itself (the engine reports no
    // check) — see F0003 → Stockfish communication.
    assert_eq!(body.status, "white-in-check", "a reply that checks White is white-in-check (B-6)");
    // 2. a black move was applied (B-8).
    assert!(body.from.is_some() && body.to.is_some(), "a move was applied (B-8)");
    Ok(())
}

// ---- TC-IT-F0003-004 --------------------------------------------------------

#[tokio::test]
async fn t40_f0003_reply_mates_white() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = seed(&base, board_3()).await?;

    let resp = opponent_move(&base, &uuid).await?;
    assert_eq!(resp.status(), 200, "the mating reply is applied (B-6)");
    let body: OpponentMoveResponse = resp.json().await?;

    // 1. the server reads the engine's `mate 1` score on the reply (B-6).
    assert_eq!(body.status, "white-in-checkmate", "a reply that mates White is white-in-checkmate (B-6)");
    // 2. the mating reply was still applied to the board (B-8).
    assert!(body.from.is_some() && body.to.is_some(), "the game-ending move is played (B-8)");
    assert_eq!(body.piece.as_deref(), Some("q"), "the mating black queen moved (B-8)");
    Ok(())
}

// ---- TC-IT-F0003-005 --------------------------------------------------------

#[tokio::test]
async fn t50_f0003_black_checkmated_no_move() -> Result<(), Err> {
    let base = base_url().await;
    let seeded = board_4();
    let uuid = seed(&base, seeded.clone()).await?;

    let resp = opponent_move(&base, &uuid).await?;
    assert_eq!(resp.status(), 200, "a game-over position is a normal 200 (B-3)");
    let body: OpponentMoveResponse = resp.json().await?;

    // 1. the engine answered (none) and Black is in check → checkmate (B-3).
    assert_eq!(body.status, "black-in-checkmate", "Black mated → black-in-checkmate (B-3)");
    // 2. no reply was applied when Black has no move (B-3).
    assert_eq!(body.from, None, "no from on a game-over result (B-3)");
    assert_eq!(body.to, None, "no to on a game-over result (B-3)");
    assert_eq!(body.piece, None, "no piece on a game-over result (B-3)");
    assert_eq!(body.capture, None, "no capture on a game-over result (B-3)");
    // 3. the stored board is left unchanged (B-3).
    assert_eq!(body.overview.board, seeded, "the board is unchanged on game over (B-3)");
    Ok(())
}

// ---- TC-IT-F0003-006 --------------------------------------------------------

#[tokio::test]
async fn t60_f0003_black_stalemated_no_move() -> Result<(), Err> {
    let base = base_url().await;
    let seeded = board_5();
    let uuid = seed(&base, seeded.clone()).await?;

    let resp = opponent_move(&base, &uuid).await?;
    assert_eq!(resp.status(), 200, "a stalemate is a normal 200 (B-3)");
    let body: OpponentMoveResponse = resp.json().await?;

    // 1. the engine answered (none) and Black is NOT in check → stalemate (B-3).
    assert_eq!(body.status, "black-in-stalemate", "Black with no move, not in check → black-in-stalemate (B-3)");
    // 2. nothing was applied and the board is unchanged (B-3).
    assert!(body.from.is_none() && body.to.is_none() && body.piece.is_none() && body.capture.is_none(), "no move on a game-over result (B-3)");
    assert_eq!(body.overview.board, seeded, "the board is unchanged on stalemate (B-3)");
    Ok(())
}

// ---- TC-IT-F0003-007 --------------------------------------------------------

#[tokio::test]
async fn t70_f0003_malformed_and_unknown() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // A body missing `uuid` → 400 before the engine (B-1).
    let resp = client
        .post(format!("{base}/opponent-move"))
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "a body missing uuid is malformed (B-1)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "invalid opponent-move request", "stable malformed-request error (B-1)");

    // A uuid naming no game → 404 before the engine (B-1).
    let unknown = Uuid::new_v4().to_string();
    let resp = opponent_move(&base, &unknown).await?;
    assert_eq!(resp.status(), 404, "an unknown game is 404 (B-1)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "unknown game", "stable unknown-game message (B-1)");
    Ok(())
}

// ---- TC-IT-F0003-008 --------------------------------------------------------

#[tokio::test]
async fn t80_f0003_cors_allow_origin() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = start_game(&base).await?;
    let origin = "http://localhost:5173";

    let resp = reqwest::Client::new()
        .post(format!("{base}/opponent-move"))
        .header("Origin", origin)
        .json(&json!({ "uuid": uuid }))
        .send()
        .await?;

    assert_eq!(resp.status(), 200, "the reply is produced under the CORS layer");
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("CORS allow-origin header present (B-10)")
        .to_str()?
        .to_string();
    assert!(acao == origin || acao == "*", "CORS permits the front dev origin (B-10), got {acao:?}");
    Ok(())
}

// ---- TC-IT-F0003-009 --------------------------------------------------------

/// Runs only where the harness can stop the Stockfish container; ignored here
/// because the other cases require the engine **up** (IT-F0003 TC-009).
#[tokio::test]
#[ignore = "requires stopping the Stockfish engine; see IT-F0003 TC-009"]
async fn t90_f0003_engine_unavailable_502() -> Result<(), Err> {
    let base = base_url().await;
    let uuid = start_game(&base).await?;

    let resp = opponent_move(&base, &uuid).await?;
    // 1. an engine transport failure surfaces as an engine fault (B-5, B-9).
    assert_eq!(resp.status(), 502, "an unreachable engine is 502 (B-5, B-9)");
    let body: ErrorBody = resp.json().await?;
    assert_eq!(body.error, "engine unavailable", "stable engine-failure message (B-9)");
    Ok(())
}

// ---- TC-IT-F0003-010 --------------------------------------------------------

#[tokio::test]
async fn t100_f0003_white_move_then_reply_compose() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();
    let uuid = start_game(&base).await?;

    // Play a real White move d2 -> d4 (the F0002 contract, already verified by
    // IT-F0002; kept here only to set up the position).
    let resp = client
        .post(format!("{base}/move-a-piece"))
        .json(&json!({ "uuid": uuid, "from": "d2", "to": "d4" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "the White move is applied (F0002 B-7)");
    let white: MoveResponse = resp.json().await?;
    assert_eq!(cell(&white.overview, "d2"), "", "d2 emptied by the White move (F0002 B-7)");
    assert_eq!(cell(&white.overview, "d4"), "P", "d4 holds the white pawn (F0002 B-7)");

    // Then ask for Black's reply against the position that move produced.
    let resp = opponent_move(&base, &uuid).await?;
    assert_eq!(resp.status(), 200, "the reply is served over the post-d4 position (B-4, B-5)");
    let body: OpponentMoveResponse = resp.json().await?;

    // 2. an ordinary reply that neither checks nor mates (B-6).
    assert_eq!(body.status, "move", "the reply after 1.d4 is an ordinary move (B-6)");
    // 3. a legal, non-capturing black reply (B-5, B-8).
    let from = body.from.as_deref().expect("from present (B-5)");
    let to = body.to.as_deref().expect("to present (B-5)");
    let piece = body.piece.as_deref().expect("piece present (B-5)");
    assert_eq!(piece, piece.to_lowercase(), "a black piece moved (B-5)");
    assert_eq!(body.capture, None, "no capture after 1.d4 (B-8)");
    // 4. the reply is applied ON TOP of the White move — the two compose (B-8).
    assert_eq!(cell(&body.overview, "d4"), "P", "White's pawn still stands on d4 (B-8)");
    assert_eq!(cell(&body.overview, from), "", "Black's source is emptied (B-8)");
    assert_eq!(cell(&body.overview, to), piece, "Black's target holds the moved piece (B-8)");
    Ok(())
}
