//! Proof-log plumbing: the closed set of stream tags (`LogFeature`), the
//! `Follower` trailer, and the `log_info_f!` / `log_warn_f!` / `log_error_f!`
//! macros that stamp every proof log with its execution context.
//!
//! See `_ai/global/proof-logs.md`. Every log in this project is a **proof log**;
//! the **`follower`** trailer — `[[session][request][stream]]` — carries the
//! three keys used to reconstruct a proof line, so it must ride on every log.
//! These macros delegate to the classic `tracing` macros and only prepend that
//! trailer, so a call site stays as short as a normal `tracing::info!`.

use std::fmt;

/// The closed, discoverable set of streams — one variant per feature
/// (proof-logs.md, "Stream codes"). Keeping every stream here means a tag can't
/// be misspelled into existence and they are all visible in one place. Add a
/// variant when you add a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFeature {
    /// Feature F0001 — start a game.
    StartAGame,
    /// Feature F0002 — move a piece.
    MoveAPiece,
    /// Feature F0003 — play with Stockfish (the opponent reply).
    PlayWithStockfish,
}

impl LogFeature {
    /// The grep-able business tag written into every proof log of this stream.
    pub const fn as_str(self) -> &'static str {
        match self {
            LogFeature::StartAGame => "START-A-GAME",
            LogFeature::MoveAPiece => "MOVE-A-PIECE",
            LogFeature::PlayWithStockfish => "PLAY-WITH-STOCKFISH",
        }
    }
}

/// The `follower` trailer that places a log into its proof line:
/// `[[session][request][stream]]` (proof-logs.md, "Anatomy of a log"). The three
/// fields are exactly the keys used to reconstruct one proof line — grep any one
/// of them to isolate a session, a request end to end, or a whole feature.
///
/// Built and rendered by the `log_*_f!` macros; you rarely name it directly.
pub struct Follower<'a> {
    /// Session id — the browser session the request belongs to.
    pub session: &'a str,
    /// Request tracking id — one request, across every service it touches.
    pub request: &'a str,
    /// Stream — which feature / business flow the log belongs to.
    pub stream: &'a str,
}

impl fmt::Display for Follower<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[[{}][{}][{}]]", self.session, self.request, self.stream)
    }
}

/// Emit an `INFO` proof log stamped with its `follower` trailer.
///
/// Arguments, in order: the **stream** (`LogFeature::_.as_str()`), the **session
/// id**, the **request tracking id**; everything after is a normal `tracing`
/// message — format string, format args, and optional extra fields
/// (`uuid = %uuid`, …). It expands to a plain `tracing::info!` carrying
/// `follower=[[session][request][stream]]`.
///
/// ```ignore
/// log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, "start-game requested");
/// log_info_f!(LogFeature::StartAGame.as_str(), session, tracking, uuid = %uuid, "start-game served");
/// ```
#[macro_export]
macro_rules! log_info_f {
    ($stream:expr, $session:expr, $request:expr, $($arg:tt)+) => {
        ::tracing::info!(
            follower = %$crate::proof_log::Follower { session: $session, request: $request, stream: $stream },
            $($arg)+
        )
    };
}

/// Emit a `WARN` proof log stamped with its `follower` trailer (a recoverable
/// problem the flow handled and continued past). Same shape as [`log_info_f!`].
#[macro_export]
macro_rules! log_warn_f {
    ($stream:expr, $session:expr, $request:expr, $($arg:tt)+) => {
        ::tracing::warn!(
            follower = %$crate::proof_log::Follower { session: $session, request: $request, stream: $stream },
            $($arg)+
        )
    };
}

/// Emit an `ERROR` proof log stamped with its `follower` trailer (a failure that
/// aborts the action). Same shape as [`log_info_f!`].
#[macro_export]
macro_rules! log_error_f {
    ($stream:expr, $session:expr, $request:expr, $($arg:tt)+) => {
        ::tracing::error!(
            follower = %$crate::proof_log::Follower { session: $session, request: $request, stream: $stream },
            $($arg)+
        )
    };
}
