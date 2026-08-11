//! Navigation and directional glyphs.

/// Rightwards arrow (U+2192).
pub const RIGHT: &str = "\u{2192}";

/// Leftwards arrow (U+2190).
pub const LEFT: &str = "\u{2190}";

/// Left-right arrow (U+2194) -- bidirectional / join.
pub const BIDI: &str = "\u{2194}";

/// Rightwards double arrow (U+21D2) -- implies / maps-to.
pub const DOUBLE_RIGHT: &str = "\u{21D2}";

/// Long rightwards arrow (U+27F6).
pub const LONG_RIGHT: &str = "\u{27F6}";

/// North east arrow (U+2197) -- external / open-out.
pub const NE: &str = "\u{2197}";

/// Rightwards arrow with hook (U+21AA) -- reply / continue.
pub const HOOK_RIGHT: &str = "\u{21AA}";

/// Leftwards arrow with hook (U+21A9).
pub const HOOK_LEFT: &str = "\u{21A9}";

/// Downwards arrow with tip rightwards (U+21B3) -- branch / child.
pub const BRANCH: &str = "\u{21B3}";

/// Arrow pointing rightwards then curving upwards (U+2934).
pub const CURVE_UP: &str = "\u{2934}";

/// Clockwise open circle arrow (U+21BB) -- reload / refresh.
pub const RELOAD: &str = "\u{21BB}";

/// Black up-pointing triangle (U+25B2) + VS15 -- collapse / expand toggle.
pub const COLLAPSE: &str = "\u{25B2}\u{FE0E}";
