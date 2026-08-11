//! Status / state glyphs for UI chrome and CLI.

/// Check mark (U+2713) + VS15 -- verified / ok (text presentation).
pub const OK: &str = "\u{2713}\u{FE0E}";

/// White heavy check mark (U+2705) -- badge-style pass (emoji colour OK).
pub const PASS: &str = "\u{2705}";

/// Ballot X (U+2717) -- fail / error.
pub const FAIL: &str = "\u{2717}";

/// Cross mark (U+274C) -- badge-style fail (emoji colour OK).
pub const REJECT: &str = "\u{274C}";

/// Multiplication X (U+2715) -- light dismiss / clear.
pub const CROSS: &str = "\u{2715}";

/// Warning sign (U+26A0) + VS15 -- warn.
pub const WARN: &str = "\u{26A0}\u{FE0E}";

/// Stopwatch (U+23F1) + VS15 -- pending / elapsed timer.
pub const TIMER: &str = "\u{23F1}\u{FE0E}";

/// Alias for [`TIMER`].
pub const PENDING: &str = TIMER;

/// Alarm clock (U+23F0) + VS15.
pub const ALARM: &str = "\u{23F0}\u{FE0E}";

/// Hourglass with flowing sand (U+23F3) + VS15 -- waiting.
pub const HOURGLASS: &str = "\u{23F3}\u{FE0E}";

/// Black circle (U+25CF) -- live / in-progress dot.
pub const LIVE: &str = "\u{25CF}";

/// Black square (U+25A0) -- stop / done.
pub const STOP: &str = "\u{25A0}";

/// Black right-pointing triangle (U+25B6) + VS15 -- play.
pub const PLAY: &str = "\u{25B6}\u{FE0E}";
