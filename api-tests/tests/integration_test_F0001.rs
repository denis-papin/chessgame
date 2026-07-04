//! Integration tests for feature F0001 — start a game (IT-F0001).
//!
//! Black-box, through the public API: each test boots the real `fisher-server`
//! Axum app on an ephemeral loopback port and drives it over HTTP with reqwest.
//! No mock, no internal-state inspection — only HTTP status, headers and JSON
//! body are observed. Every assertion ties its outcome back to the rule it
//! proves (see `_ai/features/F0001-start-a-game/IT-F0001.md`).

use std::collections::HashSet;

use serde::Deserialize;
use uuid::Uuid;

const N: usize = 50; // repeats per random case, so a single draw can't hide a violation

// ---- local deser structs (the test only knows the public wire shape) --------

#[derive(Debug, Deserialize)]
struct StartGameResponse {
    uuid: String,
    mode: String,
    overview: Overview,
}

#[derive(Debug, Deserialize)]
struct Overview {
    board: Vec<Vec<String>>,
    white: String,
    black: String,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    error: String,
}

// ---- harness & helpers ------------------------------------------------------

/// Base URL of the already-running `fisher-server`. The api-tests drive the
/// *current* server (started separately, e.g. `cargo run -p fisher-server`);
/// they no longer boot one. Override the address with `FISHER_SERVER_URL`.
fn server_url() -> String {
    std::env::var("FISHER_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:7200".to_string())
}

/// Return the running server's base URL, failing with a clear message if
/// nothing is listening. A live server answers *something* (even 404) on its
/// root; a missing one yields a connection error.
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

/// Count non-empty cells whose letter is uppercase (white) or lowercase (black).
fn count_color(ov: &Overview, white: bool) -> usize {
    ov.board
        .iter()
        .flatten()
        .filter(|cell| match cell.chars().next() {
            Some(ch) if white => ch.is_ascii_uppercase(),
            Some(ch) => ch.is_ascii_lowercase(),
            None => false,
        })
        .count()
}

/// Count cells equal to a specific piece letter.
fn count_piece(ov: &Overview, piece: &str) -> usize {
    ov.board.iter().flatten().filter(|c| c.as_str() == piece).count()
}

/// A board cell is dark when `(row + col)` is odd, so `board[7][0]` (`a1`) is
/// dark — the same parity as `squareColor` / rule F-2. Used to split a bishop
/// pair across square colours (B-11).
fn square_is_dark(row: usize, col: usize) -> bool {
    (row + col) % 2 == 1
}

/// The (row, col) cells holding a specific piece letter.
fn cells_of(ov: &Overview, piece: &str) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    for (r, row) in ov.board.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if cell == piece {
                v.push((r, c));
            }
        }
    }
    v
}

/// The set of occupied (row, col) squares.
fn occupied(ov: &Overview) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    for (r, row) in ov.board.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if !cell.is_empty() {
                v.push((r, c));
            }
        }
    }
    v
}

type Err = Box<dyn std::error::Error>;

// ---- TC-IT-F0001-001 --------------------------------------------------------

#[tokio::test]
async fn t10_f0001_default_standard_overview() -> Result<(), Err> {
    let base = base_url().await;
    let resp = reqwest::get(format!("{base}/start-game")).await?;

    // 1. success path of B-1
    assert_eq!(resp.status(), 200, "default call must return 200 (B-1)");
    let body: StartGameResponse = resp.json().await?;

    // 2. uuid is a valid v4 (B-1)
    let id = Uuid::parse_str(&body.uuid).expect("uuid parses (B-1)");
    assert_eq!(id.get_version_num(), 4, "uuid must be v4 (B-1)");

    // 3. default resolves to standard
    assert_eq!(body.mode, "standard", "default mode is standard (Inputs)");

    // 4. exact standard board (B-2)
    let ov = &body.overview;
    assert_eq!(ov.board[0], vec!["r", "n", "b", "q", "k", "b", "n", "r"], "rank 8 (B-2)");
    assert!(ov.board[1].iter().all(|c| c == "p"), "rank 7 all black pawns (B-2)");
    for r in 2..=5 {
        assert!(ov.board[r].iter().all(|c| c.is_empty()), "ranks 6-3 empty (B-2)");
    }
    assert!(ov.board[6].iter().all(|c| c == "P"), "rank 2 all white pawns (B-2)");
    assert_eq!(ov.board[7], vec!["R", "N", "B", "Q", "K", "B", "N", "R"], "rank 1 (B-2)");

    // 5. standard castling availability (B-2)
    assert_eq!(ov.white, "both", "white both (B-2)");
    assert_eq!(ov.black, "both", "black both (B-2)");
    Ok(())
}

// ---- TC-IT-F0001-002 --------------------------------------------------------

#[tokio::test]
async fn t20_f0001_explicit_standard() -> Result<(), Err> {
    let base = base_url().await;
    let resp = reqwest::get(format!("{base}/start-game?mode=standard")).await?;
    assert_eq!(resp.status(), 200);
    let body: StartGameResponse = resp.json().await?;

    assert_eq!(body.mode, "standard");
    assert_eq!(body.overview.board[0], vec!["r", "n", "b", "q", "k", "b", "n", "r"], "B-2 explicit");
    assert_eq!(body.overview.board[7], vec!["R", "N", "B", "Q", "K", "B", "N", "R"], "B-2 explicit");
    assert_eq!(body.overview.white, "both");
    assert_eq!(body.overview.black, "both");
    Ok(())
}

// ---- TC-IT-F0001-003 --------------------------------------------------------

#[tokio::test]
async fn t30_f0001_two_calls_distinct_uuid() -> Result<(), Err> {
    let base = base_url().await;
    let a: StartGameResponse = reqwest::get(format!("{base}/start-game")).await?.json().await?;
    let b: StartGameResponse = reqwest::get(format!("{base}/start-game")).await?.json().await?;
    assert_ne!(a.uuid, b.uuid, "each call creates a new game (B-1)");
    Ok(())
}

// ---- TC-IT-F0001-004 --------------------------------------------------------

#[tokio::test]
async fn t40_f0001_random_requested_piece_counts() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // (query, expected count per colour)
    let cases: &[(&str, usize)] = &[
        ("mode=random&pieces=2", 2),
        ("mode=random&pieces=4", 4),
        ("mode=random&pieces=16", 16),
        ("mode=random", 16), // omitted -> default 16
    ];

    for (query, expected) in cases {
        let mut occupied_sets: HashSet<Vec<(usize, usize)>> = HashSet::new();

        for _ in 0..N {
            let resp = client.get(format!("{base}/start-game?{query}")).send().await?;
            assert_eq!(resp.status(), 200, "random returns 200 ({query})");
            let body: StartGameResponse = resp.json().await?;
            assert_eq!(body.mode, "random", "mode echoes random ({query})");

            // 2. exact requested count for BOTH colours (B-3)
            assert_eq!(count_color(&body.overview, true), *expected, "white count == pieces ({query}, B-3)");
            assert_eq!(count_color(&body.overview, false), *expected, "black count == pieces ({query}, B-3)");

            occupied_sets.insert(occupied(&body.overview));
        }

        // 3. positions vary across draws of a fixed count (positions are random, not the count)
        assert!(occupied_sets.len() > 1, "positions must vary across {N} draws ({query})");
    }
    Ok(())
}

// ---- TC-IT-F0001-005 --------------------------------------------------------

#[tokio::test]
async fn t50_f0001_random_one_king_each() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();
    for _ in 0..N {
        let body: StartGameResponse =
            client.get(format!("{base}/start-game?mode=random")).send().await?.json().await?;
        assert_eq!(count_piece(&body.overview, "K"), 1, "exactly one white king (B-4)");
        assert_eq!(count_piece(&body.overview, "k"), 1, "exactly one black king (B-4)");
    }
    Ok(())
}

// ---- TC-IT-F0001-006 --------------------------------------------------------

#[tokio::test]
async fn t60_f0001_random_valid_8x8_cells() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();
    let valid: HashSet<&str> =
        ["p", "n", "b", "r", "q", "k", "P", "N", "B", "R", "Q", "K"].into_iter().collect();

    for _ in 0..N {
        let body: StartGameResponse =
            client.get(format!("{base}/start-game?mode=random")).send().await?.json().await?;
        let ov = &body.overview;

        // 1. 8x8 shape (B-8)
        assert_eq!(ov.board.len(), 8, "8 rows (B-8)");
        assert!(ov.board.iter().all(|row| row.len() == 8), "8 columns (B-8)");

        // 2. every cell is "" or a single valid piece letter (B-5, B-8)
        for cell in ov.board.iter().flatten() {
            if cell.is_empty() {
                continue;
            }
            assert_eq!(cell.chars().count(), 1, "one char per cell (B-5)");
            assert!(valid.contains(cell.as_str()), "valid piece letter {cell:?} (B-8)");
        }
    }
    Ok(())
}

// ---- TC-IT-F0001-007 --------------------------------------------------------

#[tokio::test]
async fn t70_f0001_random_pawn_ranks() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();
    for _ in 0..N {
        let body: StartGameResponse =
            client.get(format!("{base}/start-game?mode=random")).send().await?.json().await?;
        let ov = &body.overview;
        assert!(!ov.board[0].iter().any(|c| c == "P"), "no white pawn on rank 8 (B-6)");
        assert!(!ov.board[7].iter().any(|c| c == "p"), "no black pawn on rank 1 (B-6)");
    }
    Ok(())
}

// ---- TC-IT-F0001-008 --------------------------------------------------------

#[tokio::test]
async fn t80_f0001_random_no_castling() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();
    for _ in 0..N {
        let body: StartGameResponse =
            client.get(format!("{base}/start-game?mode=random")).send().await?.json().await?;
        assert_eq!(body.overview.white, "none", "random white no castling (B-7)");
        assert_eq!(body.overview.black, "none", "random black no castling (B-7)");
    }
    Ok(())
}

// ---- TC-IT-F0001-009 --------------------------------------------------------

#[tokio::test]
async fn t90_f0001_invalid_mode_400() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    for query in ["mode=xyz", "mode=STANDARD"] {
        let resp = client.get(format!("{base}/start-game?{query}")).send().await?;
        assert_eq!(resp.status(), 400, "unknown/cased mode rejected ({query})");
        let body: ErrorBody = resp.json().await?;
        assert_eq!(body.error, "invalid mode", "stable error message ({query})");
    }
    Ok(())
}

// ---- TC-IT-F0001-010 --------------------------------------------------------

#[tokio::test]
async fn t100_f0001_cors_allow_origin() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();
    let origin = "http://localhost:5173";

    let resp = client
        .get(format!("{base}/start-game"))
        .header("Origin", origin)
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("CORS allow-origin header present (B-9)")
        .to_str()?
        .to_string();
    assert!(acao == origin || acao == "*", "CORS permits the front dev origin (B-9), got {acao:?}");
    Ok(())
}

// ---- TC-IT-F0001-011 --------------------------------------------------------

#[tokio::test]
async fn t110_f0001_invalid_piece_count_400() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // Rejected counts.
    for query in ["mode=random&pieces=1", "mode=random&pieces=17", "mode=random&pieces=abc"] {
        let resp = client.get(format!("{base}/start-game?{query}")).send().await?;
        assert_eq!(resp.status(), 400, "bad count rejected ({query})");
        let body: ErrorBody = resp.json().await?;
        assert_eq!(body.error, "invalid piece count", "stable error message ({query})");
    }

    // Inclusive bounds still succeed.
    for query in ["mode=random&pieces=2", "mode=random&pieces=16"] {
        let resp = client.get(format!("{base}/start-game?{query}")).send().await?;
        assert_eq!(resp.status(), 200, "inclusive bound accepted ({query})");
    }
    Ok(())
}

// ---- TC-IT-F0001-012 --------------------------------------------------------

#[tokio::test]
async fn t120_f0001_random_material_caps() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // (query, expected per-colour count). 16 (explicit & default) saturates the caps.
    let cases: &[(&str, usize)] =
        &[("mode=random&pieces=2", 2), ("mode=random&pieces=8", 8), ("mode=random", 16), ("mode=random&pieces=16", 16)];

    for (query, count) in cases {
        for _ in 0..N {
            let body: StartGameResponse =
                client.get(format!("{base}/start-game?{query}")).send().await?.json().await?;
            assert_eq!(body.mode, "random", "mode echoes random ({query})");
            let ov = &body.overview;

            // (white-letter, black-letter, cap) per piece type.
            let caps: &[(&str, &str, usize)] = &[
                ("Q", "q", 1),
                ("R", "r", 2),
                ("B", "b", 2),
                ("N", "n", 2),
                ("P", "p", 8),
            ];

            // 2. no type exceeds its cap, for each colour, and exactly one king (B-10).
            assert_eq!(count_piece(ov, "K"), 1, "exactly one white king ({query}, B-10)");
            assert_eq!(count_piece(ov, "k"), 1, "exactly one black king ({query}, B-10)");
            for (w, b, cap) in caps {
                assert!(count_piece(ov, w) <= *cap, "white {w} <= {cap} ({query}, B-10)");
                assert!(count_piece(ov, b) <= *cap, "black {b} <= {cap} ({query}, B-10)");
            }

            // 3. at pieces=16 the caps are saturated: exactly the full army (B-10).
            if *count == 16 {
                for (w, b, cap) in caps {
                    assert_eq!(count_piece(ov, w), *cap, "white {w} == {cap} at 16 ({query}, B-10)");
                    assert_eq!(count_piece(ov, b), *cap, "black {b} == {cap} at 16 ({query}, B-10)");
                }
            }
        }
    }
    Ok(())
}

// ---- TC-IT-F0001-013 --------------------------------------------------------

#[tokio::test]
async fn t130_f0001_random_bishop_pair_opposite_colors() -> Result<(), Err> {
    let base = base_url().await;
    let client = reqwest::Client::new();

    // pieces=16 guarantees exactly two bishops per colour, so the pair is always present.
    for _ in 0..N {
        let body: StartGameResponse =
            client.get(format!("{base}/start-game?mode=random&pieces=16")).send().await?.json().await?;
        let ov = &body.overview;

        for (label, letter) in [("white", "B"), ("black", "b")] {
            let bishops = cells_of(ov, letter);
            // 1. exactly two bishops (cross-check with TC-012).
            assert_eq!(bishops.len(), 2, "{label} has two bishops at pieces=16 (B-10)");
            // 2. the pair sits on opposite-coloured squares (B-11).
            let (a, c) = (bishops[0], bishops[1]);
            assert_ne!(
                square_is_dark(a.0, a.1),
                square_is_dark(c.0, c.1),
                "{label} bishop pair on opposite square colours (B-11)"
            );
        }
    }
    Ok(())
}
