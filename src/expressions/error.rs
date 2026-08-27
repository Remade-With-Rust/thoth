//! Compile and match errors. No C err-buf.

extern crate alloc;

use alloc::string::String;
use core::fmt;

/// Engine error. Pattern-position is set for compile failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub position: Option<usize>,
    message: String,
}

/// Discriminated error kind (Oniguruma codes mapped to Rust).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Compile,
    InvalidEncoding,
    InvalidArgument,
    MatchStackLimit,
    RetryLimitMatch,
    RetryLimitSearch,
    SubexpCallLimit,
    ParseDepthLimit,
    Mismatch,
    NeverEndingRecursion,
}

impl Error {
    pub(crate) fn compile(position: usize, msg: &str) -> Self {
        Self {
            kind: ErrorKind::Compile,
            position: Some(position),
            message: String::from(msg),
        }
    }

    pub(crate) fn kind_msg(kind: ErrorKind, msg: &str) -> Self {
        Self {
            kind,
            position: None,
            message: String::from(msg),
        }
    }

    /// Human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.position {
            Some(p) => write!(f, "{} at byte {p}: {}", kind_name(self.kind), self.message),
            None => write!(f, "{}: {}", kind_name(self.kind), self.message),
        }
    }
}

fn kind_name(k: ErrorKind) -> &'static str {
    match k {
        ErrorKind::Compile => "compile error",
        ErrorKind::InvalidEncoding => "invalid encoding",
        ErrorKind::InvalidArgument => "invalid argument",
        ErrorKind::MatchStackLimit => "match stack limit",
        ErrorKind::RetryLimitMatch => "retry limit in match",
        ErrorKind::RetryLimitSearch => "retry limit in search",
        ErrorKind::SubexpCallLimit => "subexp-call limit",
        ErrorKind::ParseDepthLimit => "parse depth limit",
        ErrorKind::Mismatch => "mismatch",
        ErrorKind::NeverEndingRecursion => "never-ending recursion",
    }
}
