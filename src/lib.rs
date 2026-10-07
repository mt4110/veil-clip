//! 機密情報の検知とTTL処理。OS、ログ、永続化にはアクセスしません。
//! Detection and TTL logic without OS access, logging, or persistence.

pub mod detector;
pub mod engine;

use std::time::Duration;

pub const TTL: Duration = Duration::from_secs(5);
pub const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// UTF-8バイト長での検知上限 / Scanning limit in UTF-8 bytes.
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;
pub const REPLACEMENT_TEXT: &str = "[REDACTED BY VEIL-CLIP]";
