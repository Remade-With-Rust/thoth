//! rusty_expressions -- Oniguruma remade in pure Rust.
//!
//! Parse -> compile -> bytecode VM. Encoding and syntax are values on each
//! [`Regex`]. No C, no FFI, no `regex` crate.

extern crate alloc;

use alloc::vec::Vec;

use self::compile::compile;
use self::opcode::Program;
use self::parse::parse;

mod ast;
mod callout;
mod compile;
pub mod count;
mod encoding;
mod encoding_cjk;
mod error;
mod exec;
mod opcode;
mod optimize;
mod ops;
mod param;
mod parse;
mod region;
mod scan;
mod set;
mod syntax;
mod ucd16;
mod unicode;

#[cfg(feature = "compat")]
#[cfg_attr(docsrs, doc(cfg(feature = "compat")))]
pub mod compat;

pub use callout::{CalloutCtx, CalloutDir, CalloutFn, CalloutResult};
pub use encoding::Encoding;
pub use error::{Error, ErrorKind};
pub use ops::{find_all, find_all_str, format_matches, is_match_str};
pub use param::{
    MatchParam, DEFAULT_MATCH_STACK_LIMIT, DEFAULT_RETRY_LIMIT_IN_MATCH,
    DEFAULT_RETRY_LIMIT_IN_SEARCH,
};
pub use region::{CaptureTree, Region};
pub use scan::scan;
pub use set::RegSet;
pub use syntax::{sql_syntax, Options, Syntax};
pub use optimize::ReqLit;
pub use unicode::UserProperty;

/// Compiled regular expression. `Send + Sync`.
#[derive(Clone, Debug)]
pub struct Regex {
    prog: Program,
    enc: Encoding,
    options: Options,
    syntax: Syntax,
    user_props: Vec<UserProperty>,
}

impl Regex {
    /// Compile `pattern` (encoding `enc`).
    pub fn new(
        pattern: impl AsRef<[u8]>,
        options: Options,
        enc: Encoding,
        syntax: Syntax,
    ) -> Result<Self, Error> {
        let pattern = pattern.as_ref();
        let parsed = parse(pattern, enc, syntax, options)?;
        let mut prog = compile(&parsed);
        let options = options.union(syntax.options);
        exec::analyze(&mut prog, enc, options, &[]);
        prog.lead = exec::compute_lead(&prog, enc, options, &[]);
        Ok(Self {
            prog,
            enc,
            options,
            syntax,
            user_props: Vec::new(),
        })
    }

    /// UTF-8 pattern + haystack convenience.
    pub fn new_str(pattern: &str, options: Options, syntax: Syntax) -> Result<Self, Error> {
        Self::new(pattern.as_bytes(), options, Encoding::UTF8, syntax)
    }

    pub fn encoding(&self) -> Encoding {
        self.enc
    }

    pub fn options(&self) -> Options {
        self.options
    }

    pub fn syntax(&self) -> Syntax {
        self.syntax
    }

    /// Attach a user-defined Unicode property (`onig_unicode_define_user_property`).
    pub fn define_user_property(&mut self, prop: UserProperty) {
        self.user_props.push(prop);
        // A user property changes what a codepoint matches, so every table
        // derived from class membership -- the bitmaps, the required literal
        // and the repeat shapes, not just the first-byte filter -- has to be
        // rebuilt. Refreshing only `lead` left the required-literal filter
        // skipping past real matches.
        exec::analyze(&mut self.prog, self.enc, self.options, &self.user_props);
        self.prog.lead = exec::compute_lead(&self.prog, self.enc, self.options, &self.user_props);
    }

    pub fn is_match(&self, hay: impl AsRef<[u8]>) -> Result<bool, Error> {
        Ok(self.search(hay)?.is_some())
    }

    pub fn search(&self, hay: impl AsRef<[u8]>) -> Result<Option<Region>, Error> {
        self.search_param(hay.as_ref(), &MatchParam::default())
    }

    pub fn search_param(&self, hay: &[u8], param: &MatchParam) -> Result<Option<Region>, Error> {
        self.search_range_param(hay, 0, hay.len(), param)
    }

    pub fn search_range_param(
        &self,
        hay: &[u8],
        start: usize,
        range: usize,
        param: &MatchParam,
    ) -> Result<Option<Region>, Error> {
        if start > hay.len() || range > hay.len() {
            return Err(Error::kind_msg(
                ErrorKind::InvalidArgument,
                "search range",
            ));
        }
        exec::search(
            &self.prog,
            hay,
            self.enc,
            self.options,
            start,
            range,
            param,
            &self.user_props,
        )
    }

    /// Match only at `at` (Oniguruma `onig_match`).
    pub fn find_at(&self, hay: &[u8], at: usize) -> Result<Option<Region>, Error> {
        exec::match_at(
            &self.prog,
            hay,
            self.enc,
            self.options,
            at,
            at,
            &MatchParam::default(),
            &self.user_props,
        )
    }

    /// The byte sequence every match must contain, if the analyzer found one.
    ///
    /// Exposed so `tools/onig-bench --example reqlit` can report what the
    /// search will actually use.
    pub fn required_literal(&self) -> Option<&ReqLit> {
        self.prog.req_lit.as_ref()
    }

    pub fn capture_count(&self) -> usize {
        self.prog.capture_count
    }
}
