//! Pattern parser (Oniguruma `regparse`).

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::ast::{
    concat, alt, AbsentKind, Anchor, Backref, CallTarget, CharClass, ClassItem, Cond, Node,
};
use super::callout::CalloutDir;
use super::encoding::Encoding;
use super::error::{Error, ErrorKind};
use super::syntax::{self, behavior, op, op2, Options, Syntax};

pub struct ParseResult {
    pub root: Node,
    pub capture_count: usize,
    pub names: Vec<Option<String>>,
    pub has_named: bool,
    pub options: Options,
}

/// Does the pattern contain a named group anywhere?
///
/// Oniguruma decides this for the whole pattern before it starts capturing:
/// once any named group exists, a plain `(...)` stops capturing unless
/// `CAPTURE_GROUP` is set. Deciding it as we go made `(x)(?<a>y)` capture
/// `(x)` as group 1, which libonig does not.
fn scan_has_named_group(pat: &[u8], syntax: Syntax) -> bool {
    if !syntax.has_op2(op2::QMARK_LT_NAMED_GROUP) {
        return false;
    }
    let mut i = 0usize;
    let mut in_class = false;
    while i < pat.len() {
        match pat[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'[' if !in_class => in_class = true,
            b']' if in_class => in_class = false,
            b'(' if !in_class => {
                let rest = &pat[i..];
                // (?<name>  -- but not the look-behinds (?<= and (?<!
                if rest.len() > 3
                    && rest[1] == b'?'
                    && rest[2] == b'<'
                    && rest[3] != b'='
                    && rest[3] != b'!'
                {
                    return true;
                }
                // (?'name'
                if rest.len() > 2 && rest[1] == b'?' && rest[2] == b'\'' {
                    return true;
                }
                // (?@<name>  -- named capture-history group
                if rest.len() > 4
                    && rest[1] == b'?'
                    && rest[2] == b'@'
                    && rest[3] == b'<'
                    && rest[4] != b'='
                    && rest[4] != b'!'
                {
                    return true;
                }
                // (?P<name>
                if rest.len() > 3 && rest[1] == b'?' && rest[2] == b'P' && rest[3] == b'<' {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

pub fn parse(
    pattern: &[u8],
    enc: Encoding,
    syntax: Syntax,
    options: Options,
) -> Result<ParseResult, Error> {
    let codes = decode_pattern(pattern, enc)?;
    let mut p = Parser {
        pat: pattern,
        codes,
        enc,
        syntax,
        options,
        i: 0,
        capture_count: 1,
        names: alloc::vec![None],
        has_named: scan_has_named_group(pattern, syntax),
        depth: 0,
        group_stack: Vec::new(),
    };
    let root = p.parse_alts()?;
    if p.i != p.codes.len() && p.peek_byte() == Some(b')') && !syntax.has_behavior(behavior::ALLOW_UNMATCHED_CLOSE_SUBEXP)
    {
        return Err(Error::compile(p.i, "unmatched close parenthesis"));
    }
    check_never_ending_recursion(&root, &p.names)?;
    Ok(ParseResult {
        root,
        capture_count: p.capture_count,
        names: p.names,
        has_named: p.has_named,
        options: p.options,
    })
}

struct Parser<'a> {
    pat: &'a [u8],
    /// Pattern decoded to (byte offset, code point) so syntax peeks are encoding-aware.
    codes: Vec<(usize, u32)>,
    #[allow(dead_code)]
    enc: Encoding,
    syntax: Syntax,
    options: Options,
    i: usize,
    capture_count: usize,
    names: Vec<Option<String>>,
    has_named: bool,
    depth: usize,
    group_stack: Vec<usize>,
}

fn decode_pattern(pat: &[u8], enc: Encoding) -> Result<Vec<(usize, u32)>, Error> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < pat.len() {
        let n = enc.mbc_len(&pat[i..])?;
        let code = enc.mbc_to_code(&pat[i..])?;
        out.push((i, code));
        i += n;
    }
    Ok(out)
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> Error {
        let pos = self.codes.get(self.i).map(|x| x.0).unwrap_or(self.pat.len());
        Error::compile(pos, msg)
    }

    fn peek_code(&self) -> Option<u32> {
        self.codes.get(self.i).map(|x| x.1)
    }

    fn peek_code_at(&self, n: usize) -> Option<u32> {
        self.codes.get(self.i + n).map(|x| x.1)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.peek_code().and_then(|c| u8::try_from(c).ok())
    }

    fn bump(&mut self) -> Result<u32, Error> {
        let code = self.peek_code().ok_or_else(|| self.err("unexpected end of pattern"))?;
        self.i += 1;
        Ok(code)
    }

    fn skip_ws_if_extend(&mut self) -> Result<(), Error> {
        if !self.options.contains(Options::EXTEND) {
            return Ok(());
        }
        while let Some(b) = self.peek_byte() {
            if b == b'#' {
                while self.peek_byte().is_some() && self.peek_byte() != Some(b'\n') {
                    self.i += 1;
                }
            } else if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c') {
                self.i += 1;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Nesting depth the parser will accept.
    ///
    /// This is a **native stack** limit, not a taste limit. Parsing,
    /// compiling and the compile-time analysis walks are all recursive over
    /// the pattern's structure, so nesting depth is call depth. Measured
    /// ceiling on a 1 MB main-thread stack is ~400 nested groups; this leaves
    /// a 2x margin, and callers on smaller stacks (embedded, wasm) get the
    /// benefit. Real patterns nest a handful of levels.
    ///
    /// The previous value of 4096 was above the ceiling it was meant to
    /// protect -- the same mistake as a match-stack limit set above what the
    /// native stack can hold: the guard existed but could never fire first.
    const MAX_PARSE_DEPTH: usize = 200;

    fn check_depth(&self) -> Result<(), Error> {
        if self.depth > Self::MAX_PARSE_DEPTH {
            return Err(Error::kind_msg(
                super::error::ErrorKind::ParseDepthLimit,
                "parse depth",
            ));
        }
        Ok(())
    }

    fn parse_alts(&mut self) -> Result<Node, Error> {
        self.depth += 1;
        self.check_depth()?;
        let mut alts = Vec::new();
        loop {
            self.skip_ws_if_extend()?;
            alts.push(self.parse_concat()?);
            self.skip_ws_if_extend()?;
            let bar = if self.syntax.has_op(op::VBAR_ALT) && self.peek_byte() == Some(b'|') {
                true
            } else if self.syntax.has_op(op::ESC_VBAR_ALT) && self.peek_byte() == Some(b'\\') {
                self.peek_code_at(1) == Some(u32::from(b'|'))
            } else {
                false
            };
            if !bar {
                break;
            }
            if self.peek_byte() == Some(b'\\') {
                self.i += 2;
            } else {
                self.i += 1;
            }
        }
        self.depth -= 1;
        Ok(alt(alts))
    }

    fn parse_concat(&mut self) -> Result<Node, Error> {
        let mut nodes = Vec::new();
        loop {
            self.skip_ws_if_extend()?;
            if self.i >= self.codes.len() {
                break;
            }
            match self.peek_byte() {
                Some(b'|') if self.syntax.has_op(op::VBAR_ALT) => break,
                Some(b')') => break,
                Some(b'\\') if self.syntax.has_op(op::ESC_VBAR_ALT)
                    && self.peek_code_at(1) == Some(u32::from(b'|')) =>
                {
                    break;
                }
                _ => {}
            }
            nodes.push(self.parse_quantified()?);
        }
        Ok(concat(nodes))
    }

    fn parse_quantified(&mut self) -> Result<Node, Error> {
        let atom = self.parse_atom()?;
        self.skip_ws_if_extend()?;
        let (min, max, greedy, possessive, opt_exact) = match self.try_quant()? {
            Some(q) => q,
            None => return Ok(atom),
        };
        let inner = if opt_exact {
            Node::Repeat {
                inner: Box::new(atom),
                min,
                max,
                greedy: true,
                possessive: false,
            }
        } else {
            atom
        };
        Ok(Node::Repeat {
            inner: Box::new(inner),
            min: if opt_exact { 0 } else { min },
            max: if opt_exact { Some(1) } else { max },
            greedy,
            possessive,
        })
    }

    fn try_quant(&mut self) -> Result<Option<(u32, Option<u32>, bool, bool, bool)>, Error> {
        let b = match self.peek_byte() {
            Some(b) => b,
            None => return Ok(None),
        };
        let (min, max) = if b == b'*' && self.syntax.has_op(op::ASTERISK_ZERO_INF) {
            self.i += 1;
            (0, None)
        } else if b == b'+' && self.syntax.has_op(op::PLUS_ONE_INF) {
            self.i += 1;
            (1, None)
        } else if b == b'?' && self.syntax.has_op(op::QMARK_ZERO_ONE) {
            self.i += 1;
            (0, Some(1))
        } else if b == b'{' && self.syntax.has_op(op::BRACE_INTERVAL) {
            self.parse_brace()?
        } else if b == b'\\' {
            if self.syntax.has_op(op::ESC_ASTERISK_ZERO_INF) && self.peek_code_at(1) == Some(u32::from(b'*'))
            {
                self.i += 2;
                (0, None)
            } else if self.syntax.has_op(op::ESC_PLUS_ONE_INF) && self.peek_code_at(1) == Some(u32::from(b'+'))
            {
                self.i += 2;
                (1, None)
            } else if self.syntax.has_op(op::ESC_QMARK_ZERO_ONE) && self.peek_code_at(1) == Some(u32::from(b'?'))
            {
                self.i += 2;
                (0, Some(1))
            } else if self.syntax.has_op(op::ESC_BRACE_INTERVAL) && self.peek_code_at(1) == Some(u32::from(b'{'))
            {
                self.i += 1;
                self.parse_brace()?
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        };
        let mut greedy = true;
        let mut possessive = false;
        if self.peek_byte() == Some(b'?') && self.syntax.has_op(op::QMARK_NON_GREEDY) {
            // Oniguruma FIXED_INTERVAL_IS_GREEDY_ONLY: `{n}?` is `(?:{n})?`, not lazy `{n}`.
            if self.syntax.has_behavior(behavior::FIXED_INTERVAL_IS_GREEDY_ONLY)
                && min == max.unwrap_or(min)
                && max.is_some()
            {
                self.i += 1;
                return Ok(Some((min, max, true, false, true)));
            }
            self.i += 1;
            greedy = false;
        } else if self.peek_byte() == Some(b'+')
            && (self.syntax.has_op2(op2::PLUS_POSSESSIVE_REPEAT)
                || self.syntax.has_op2(op2::PLUS_POSSESSIVE_INTERVAL))
        {
            self.i += 1;
            possessive = true;
        }
        Ok(Some((min, max, greedy, possessive, false)))
    }

    fn parse_brace(&mut self) -> Result<(u32, Option<u32>), Error> {
        if self.peek_byte() != Some(b'{') {
            return Err(self.err("expected '{'"));
        }
        let brace_at = self.i;
        self.i += 1;
        let lower_s = self.read_ascii_digits();
        let mut lower: Option<u32> = if lower_s.is_empty() {
            None
        } else {
            Some(lower_s.parse().map_err(|_| self.err("bad repeat"))?)
        };
        let mut upper: Option<u32> = lower;
        if self.peek_byte() == Some(b',') {
            self.i += 1;
            let us = self.read_ascii_digits();
            upper = if us.is_empty() {
                None
            } else {
                Some(us.parse().map_err(|_| self.err("bad repeat"))?)
            };
            if lower.is_none() {
                if self.syntax.has_behavior(behavior::ALLOW_INTERVAL_LOW_ABBREV) {
                    lower = Some(0);
                } else {
                    return Err(self.err("invalid repeat range"));
                }
            }
        }
        if self.peek_byte() != Some(b'}') {
            if self.syntax.has_op(op::ESC_BRACE_INTERVAL)
                && self.peek_byte() == Some(b'\\')
                && self.peek_code_at(1) == Some(u32::from(b'}'))
            {
                self.i += 2;
            } else if self.syntax.has_behavior(behavior::ALLOW_INVALID_INTERVAL) {
                self.i = brace_at;
                return Ok((1, Some(1)));
            } else {
                return Err(self.err("incomplete {n,m}"));
            }
        } else {
            self.i += 1;
        }
        let min = lower.unwrap_or(0);
        if let Some(u) = upper {
            if u < min {
                return Err(self.err("upper smaller than lower"));
            }
        }
        Ok((min, upper))
    }

    fn parse_atom(&mut self) -> Result<Node, Error> {
        self.skip_ws_if_extend()?;
        let Some(code) = self.peek_code() else {
            return Ok(Node::Empty);
        };
        if let Ok(b) = u8::try_from(code) {
            if self.syntax.has_op(op::VARIABLE_META_CHARACTERS) {
                if Some(u32::from(b)) == Some(self.syntax.meta_anychar).filter(|&c| c != 0) {
                    self.i += 1;
                    return Ok(Node::Any {
                        newline: self.options.contains(Options::MULTILINE),
                    });
                }
                if Some(u32::from(b)) == Some(self.syntax.meta_anychar_anytime).filter(|&c| c != 0) {
                    self.i += 1;
                    return Ok(Node::Repeat {
                        inner: Box::new(Node::Any {
                            newline: self.options.contains(Options::MULTILINE),
                        }),
                        min: 0,
                        max: None,
                        greedy: true,
                        possessive: false,
                    });
                }
            }
            match b {
                b'.' if self.syntax.has_op(op::DOT_ANYCHAR) => {
                    self.i += 1;
                    return Ok(Node::Any {
                        newline: self.options.contains(Options::MULTILINE),
                    });
                }
                b'^' if self.syntax.has_op(op::LINE_ANCHOR) => {
                    self.i += 1;
                    return Ok(if self.options.contains(Options::SINGLELINE) {
                        Node::Anchor(Anchor::Bos)
                    } else {
                        Node::Anchor(Anchor::Bol)
                    });
                }
                b'$' if self.syntax.has_op(op::LINE_ANCHOR) => {
                    self.i += 1;
                    return Ok(if self.options.contains(Options::SINGLELINE) {
                        Node::Anchor(Anchor::EosNl)
                    } else {
                        Node::Anchor(Anchor::Eol)
                    });
                }
                b'[' if self.syntax.has_op(op::BRACKET_CC) => return self.parse_class(),
                b'(' if self.syntax.has_op(op::LPAREN_SUBEXP) => return self.parse_group(),
                b'\\' => return self.parse_escape(),
                _ => {}
            }
        }
        let c = self.bump()?;
        Ok(Node::Char(c))
    }

    fn parse_group(&mut self) -> Result<Node, Error> {
        self.i += 1;
        if self.peek_byte() == Some(b'?') && self.syntax.has_op2(op2::QMARK_GROUP_EFFECT) {
            self.i += 1;
            return self.parse_qmark_group();
        }
        if self.peek_byte() == Some(b'*') && self.syntax.has_op2(op2::ASTERISK_CALLOUT_NAME) {
            return self.parse_named_callout();
        }
        self.parse_capturing(None, false)
    }

    fn parse_capturing(&mut self, name: Option<String>, history: bool) -> Result<Node, Error> {
        let capture_unnamed = !self.has_named || self.options.contains(Options::CAPTURE_GROUP);
        let dont = self.options.contains(Options::DONT_CAPTURE_GROUP);
        let capturing = if name.is_some() {
            true
        } else if dont {
            false
        } else {
            capture_unnamed
        };
        let idx = if capturing {
            let i = self.capture_count;
            self.capture_count += 1;
            self.names.push(name.clone());
            if name.is_some() {
                self.has_named = true;
            }
            Some(i)
        } else {
            None
        };
        let inner = self.parse_alts()?;
        if self.peek_byte() != Some(b')') {
            return Err(self.err("unmatched parenthesis"));
        }
        self.i += 1;
        Ok(match idx {
            Some(index) => Node::Capture {
                index,
                name,
                inner: Box::new(inner),
                history,
            },
            None => Node::Group(Box::new(inner)),
        })
    }

    fn parse_qmark_group(&mut self) -> Result<Node, Error> {
        let b = self.peek_byte().ok_or_else(|| self.err("unterminated group"))?;
        match b {
            b':' => {
                self.i += 1;
                let inner = self.parse_alts()?;
                self.expect_close()?;
                Ok(Node::Group(Box::new(inner)))
            }
            b'#' => {
                self.i += 1;
                while self.peek_byte().is_some() && self.peek_byte() != Some(b')') {
                    self.i += 1;
                }
                self.expect_close()?;
                Ok(Node::Empty)
            }
            b'=' => {
                self.i += 1;
                let inner = self.parse_alts()?;
                self.expect_close()?;
                Ok(Node::Look {
                    behind: false,
                    negative: false,
                    inner: Box::new(inner),
                })
            }
            b'!' => {
                self.i += 1;
                let inner = self.parse_alts()?;
                self.expect_close()?;
                Ok(Node::Look {
                    behind: false,
                    negative: true,
                    inner: Box::new(inner),
                })
            }
            b'>' => {
                self.i += 1;
                let inner = self.parse_alts()?;
                self.expect_close()?;
                Ok(Node::Atomic(Box::new(inner)))
            }
            b'~' if self.syntax.has_op2(op2::QMARK_TILDE_ABSENT_GROUP) => self.parse_absent(),
            b'(' if self.syntax.has_op2(op2::QMARK_LPAREN_IF_ELSE) => self.parse_conditional(),
            b'<' => self.parse_lt_group(),
            b'\'' if self.syntax.has_op2(op2::QMARK_LT_NAMED_GROUP) => {
                self.i += 1;
                let name = self.read_name(b'\'')?;
                self.parse_capturing(Some(name), false)
            }
            b'@' => {
                self.i += 1;
                self.parse_capturing(None, true)
            }
            b'P' if self.syntax.has_op2(op2::QMARK_CAPITAL_P_NAME) => self.parse_python_name(),
            b'{' if self.syntax.has_op2(op2::QMARK_BRACE_CALLOUT_CONTENTS) => {
                self.parse_contents_callout()
            }
            b'R' | b'0'..=b'9' | b'&' if self.syntax.has_op2(op2::QMARK_PERL_SUBEXP_CALL) => {
                self.parse_perl_call()
            }
            _ => self.parse_options_group(),
        }
    }

    fn parse_lt_group(&mut self) -> Result<Node, Error> {
        self.i += 1;
        match self.peek_byte() {
            Some(b'=') => {
                self.i += 1;
                let inner = self.parse_alts()?;
                self.expect_close()?;
                Ok(Node::Look {
                    behind: true,
                    negative: false,
                    inner: Box::new(inner),
                })
            }
            Some(b'!') => {
                self.i += 1;
                let inner = self.parse_alts()?;
                self.expect_close()?;
                Ok(Node::Look {
                    behind: true,
                    negative: true,
                    inner: Box::new(inner),
                })
            }
            _ if self.syntax.has_op2(op2::QMARK_LT_NAMED_GROUP) => {
                let name = self.read_name(b'>')?;
                self.parse_capturing(Some(name), false)
            }
            _ => Err(self.err("invalid group")),
        }
    }

    fn parse_python_name(&mut self) -> Result<Node, Error> {
        self.i += 1;
        match self.peek_byte() {
            Some(b'<') => {
                self.i += 1;
                let name = self.read_name(b'>')?;
                self.parse_capturing(Some(name), false)
            }
            Some(b'=') => {
                self.i += 1;
                let name = self.read_name(b')')?;
                Ok(Node::Backref(Backref::Name(name)))
            }
            _ => Err(self.err("invalid (?P")),
        }
    }

    fn parse_perl_call(&mut self) -> Result<Node, Error> {
        if self.peek_byte() == Some(b'R') {
            self.i += 1;
            self.expect_close()?;
            return Ok(Node::Call(CallTarget::Whole));
        }
        if self.peek_byte() == Some(b'&') {
            self.i += 1;
            let name = self.read_name(b')')?;
            return Ok(Node::Call(CallTarget::Name(name)));
        }
        let n = self.read_signed_int()?;
        self.expect_close()?;
        Ok(Node::Call(CallTarget::Number(n)))
    }

    fn parse_absent(&mut self) -> Result<Node, Error> {
        self.i += 1;
        if self.peek_byte() == Some(b'|') {
            self.i += 1;
            if self.peek_byte() == Some(b')') {
                self.i += 1;
                return Ok(Node::Absent {
                    stopper: Box::new(Node::Empty),
                    expr: None,
                    kind: AbsentKind::Clear,
                });
            }
            let stopper = self.parse_alts()?;
            if self.peek_byte() == Some(b'|') {
                self.i += 1;
                let expr = self.parse_alts()?;
                self.expect_close()?;
                return Ok(Node::Absent {
                    stopper: Box::new(stopper),
                    expr: Some(Box::new(expr)),
                    kind: AbsentKind::Expression,
                });
            }
            self.expect_close()?;
            return Ok(Node::Absent {
                stopper: Box::new(stopper),
                expr: None,
                kind: AbsentKind::Stopper,
            });
        }
        let inner = self.parse_alts()?;
        self.expect_close()?;
        Ok(Node::Absent {
            stopper: Box::new(inner),
            expr: None,
            kind: AbsentKind::Repeater,
        })
    }

    fn parse_conditional(&mut self) -> Result<Node, Error> {
        self.i += 1;
        let cond = if self.peek_byte() == Some(b'<') || self.peek_byte() == Some(b'\'') {
            let q = self.bump()? as u8;
            let end = if q == b'<' { b'>' } else { b'\'' };
            let name = self.read_name(end)?;
            Cond::Name(name)
        } else if self.peek_byte().map(|b| b.is_ascii_digit() || b == b'-' || b == b'+') == Some(true)
        {
            let n = self.read_signed_int()?;
            Cond::Group(n.max(0) as usize)
        } else {
            let inner = self.parse_alts()?;
            Cond::Expr(Box::new(inner))
        };
        if self.peek_byte() != Some(b')') {
            return Err(self.err("invalid conditional"));
        }
        self.i += 1;
        let then_n = self.parse_concat()?;
        let else_n = if self.peek_byte() == Some(b'|') {
            self.i += 1;
            Some(Box::new(self.parse_concat()?))
        } else {
            None
        };
        self.expect_close()?;
        Ok(Node::Conditional {
            cond,
            then_n: Box::new(then_n),
            else_n,
        })
    }

    fn parse_options_group(&mut self) -> Result<Node, Error> {
        let mut set = Options::NONE;
        let mut clear = Options::NONE;
        let mut on = true;
        loop {
            match self.peek_byte() {
                Some(b'i') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::IGNORECASE);
                    } else {
                        clear = clear.union(Options::IGNORECASE);
                    }
                }
                Some(b'm') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::MULTILINE);
                    } else {
                        clear = clear.union(Options::MULTILINE);
                    }
                }
                Some(b'x') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::EXTEND);
                    } else {
                        clear = clear.union(Options::EXTEND);
                    }
                }
                Some(b's') if self.syntax.has_op2(op2::OPTION_PERL) => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::MULTILINE);
                    } else {
                        clear = clear.union(Options::MULTILINE);
                    }
                }
                Some(b'W') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::WORD_IS_ASCII);
                    } else {
                        clear = clear.union(Options::WORD_IS_ASCII);
                    }
                }
                Some(b'D') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::DIGIT_IS_ASCII);
                    } else {
                        clear = clear.union(Options::DIGIT_IS_ASCII);
                    }
                }
                Some(b'S') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::SPACE_IS_ASCII);
                    } else {
                        clear = clear.union(Options::SPACE_IS_ASCII);
                    }
                }
                Some(b'P') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::POSIX_IS_ASCII);
                    } else {
                        clear = clear.union(Options::POSIX_IS_ASCII);
                    }
                }
                Some(b'C') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::DONT_CAPTURE_GROUP);
                    } else {
                        clear = clear.union(Options::DONT_CAPTURE_GROUP);
                    }
                }
                Some(b'I') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::IGNORECASE_IS_ASCII);
                    } else {
                        clear = clear.union(Options::IGNORECASE_IS_ASCII);
                    }
                }
                Some(b'L') => {
                    self.i += 1;
                    if on {
                        set = set.union(Options::FIND_LONGEST);
                    } else {
                        clear = clear.union(Options::FIND_LONGEST);
                    }
                }
                Some(b'-') => {
                    self.i += 1;
                    on = false;
                }
                Some(b':') => {
                    self.i += 1;
                    let saved = self.options;
                    self.options = self.options.union(set).difference(clear);
                    let inner = self.parse_alts()?;
                    self.options = saved;
                    self.expect_close()?;
                    return Ok(Node::Options {
                        set,
                        clear,
                        inner: Box::new(inner),
                    });
                }
                Some(b')') => {
                    self.i += 1;
                    self.options = self.options.union(set).difference(clear);
                    let inner = self.parse_concat()?;
                    return Ok(Node::Options {
                        set,
                        clear,
                        inner: Box::new(inner),
                    });
                }
                Some(b'y') => {
                    self.i += 1;
                    if self.peek_byte() == Some(b'{') {
                        self.i += 1;
                        if self.peek_byte() == Some(b'w') {
                            self.i += 1;
                            set = set.union(Options::TEXT_SEGMENT_WORD);
                        } else if self.peek_byte() == Some(b'g') {
                            self.i += 1;
                            set = set.union(Options::TEXT_SEGMENT_EXTENDED_GRAPHEME_CLUSTER);
                        }
                        if self.peek_byte() == Some(b'}') {
                            self.i += 1;
                        }
                    }
                }
                _ => return Err(self.err("undefined group option")),
            }
        }
    }

    fn parse_contents_callout(&mut self) -> Result<Node, Error> {
        self.i += 1;
        let mut body = String::new();
        let mut depth = 1;
        while self.peek_code().is_some() && depth > 0 {
            match self.peek_byte() {
                Some(b'{') => {
                    depth += 1;
                    if depth > 0 {
                        body.push('{');
                    }
                    self.i += 1;
                }
                Some(b'}') => {
                    depth -= 1;
                    if depth > 0 {
                        body.push('}');
                        self.i += 1;
                    }
                }
                _ => {
                    if let Some(c) = char::from_u32(self.bump()?) {
                        body.push(c);
                    }
                }
            }
        }
        if self.peek_byte() == Some(b'}') {
            self.i += 1;
        }
        let mut dir = CalloutDir::Progress;
        let mut tag = None;
        if self.peek_byte() == Some(b'[') {
            self.i += 1;
            tag = Some(self.read_name(b']')?);
        }
        match self.peek_byte() {
            Some(b'X') => {
                dir = CalloutDir::Both;
                self.i += 1;
            }
            Some(b'<') => {
                dir = CalloutDir::Retraction;
                self.i += 1;
            }
            Some(b'>') => {
                dir = CalloutDir::Progress;
                self.i += 1;
            }
            _ => {}
        }
        self.expect_close()?;
        Ok(Node::Callout {
            named: false,
            name: String::new(),
            args: String::new(),
            tag,
            body,
            dir,
        })
    }

    fn parse_named_callout(&mut self) -> Result<Node, Error> {
        self.i += 1;
        let name = self.read_ident()?;
        let mut args = String::new();
        let mut tag = None;
        if self.peek_byte() == Some(b'[') {
            self.i += 1;
            tag = Some(self.read_name(b']')?);
        }
        if self.peek_byte() == Some(b'{') {
            self.i += 1;
            args = self.collect_until(b'}');
            if self.peek_byte() == Some(b'}') {
                self.i += 1;
            }
        }
        self.expect_close()?;
        Ok(Node::Callout {
            named: true,
            name,
            args,
            tag,
            body: String::new(),
            dir: CalloutDir::Progress,
        })
    }

    fn parse_escape(&mut self) -> Result<Node, Error> {
        self.i += 1;
        if self.syntax.has_op2(op2::INEFFECTIVE_ESCAPE) {
            return Ok(Node::Char(self.syntax.meta_escape));
        }
        let b = self.peek_byte().ok_or_else(|| self.err("end at escape"))?;
        self.i += 1;
        match b {
            b'n' if self.syntax.has_op(op::ESC_CONTROL_CHARS) => Ok(Node::Char(0x0a)),
            b't' if self.syntax.has_op(op::ESC_CONTROL_CHARS) => Ok(Node::Char(0x09)),
            b'r' if self.syntax.has_op(op::ESC_CONTROL_CHARS) => Ok(Node::Char(0x0d)),
            b'f' if self.syntax.has_op(op::ESC_CONTROL_CHARS) => Ok(Node::Char(0x0c)),
            b'a' if self.syntax.has_op(op::ESC_CONTROL_CHARS) => Ok(Node::Char(0x07)),
            b'e' if self.syntax.has_op(op::ESC_CONTROL_CHARS) => Ok(Node::Char(0x1b)),
            b'v' if self.syntax.has_op2(op2::ESC_V_VTAB) || self.syntax.has_op(op::ESC_CONTROL_CHARS) => {
                Ok(Node::Char(0x0b))
            }
            b'A' if self.syntax.has_op(op::ESC_AZ_BUF_ANCHOR) => Ok(Node::Anchor(Anchor::Bos)),
            b'Z' if self.syntax.has_op(op::ESC_AZ_BUF_ANCHOR) => Ok(Node::Anchor(Anchor::EosNl)),
            b'z' if self.syntax.has_op(op::ESC_AZ_BUF_ANCHOR) => Ok(Node::Anchor(Anchor::Eos)),
            b'G' if self.syntax.has_op(op::ESC_CAPITAL_G_BEGIN_ANCHOR) => Ok(Node::Anchor(Anchor::G)),
            b'b' if self.syntax.has_op(op::ESC_B_WORD_BOUND) => Ok(Node::Anchor(Anchor::WordBound)),
            b'B' if self.syntax.has_op(op::ESC_B_WORD_BOUND) => Ok(Node::Anchor(Anchor::NotWordBound)),
            b'<' if self.syntax.has_op(op::ESC_LTGT_WORD_BEGIN_END) => Ok(Node::Anchor(Anchor::WordBegin)),
            b'>' if self.syntax.has_op(op::ESC_LTGT_WORD_BEGIN_END) => Ok(Node::Anchor(Anchor::WordEnd)),
            b'w' if self.syntax.has_op(op::ESC_W_WORD) => Ok(class_ctype(ClassItem::Word { neg: false })),
            b'W' if self.syntax.has_op(op::ESC_W_WORD) => Ok(class_ctype(ClassItem::Word { neg: true })),
            b'd' if self.syntax.has_op(op::ESC_D_DIGIT) => Ok(class_ctype(ClassItem::Digit { neg: false })),
            b'D' if self.syntax.has_op(op::ESC_D_DIGIT) => Ok(class_ctype(ClassItem::Digit { neg: true })),
            b's' if self.syntax.has_op(op::ESC_S_WHITE_SPACE) => {
                Ok(class_ctype(ClassItem::Space { neg: false }))
            }
            b'S' if self.syntax.has_op(op::ESC_S_WHITE_SPACE) => {
                Ok(class_ctype(ClassItem::Space { neg: true }))
            }
            b'h' if self.syntax.has_op2(op2::ESC_H_XDIGIT) => {
                Ok(class_ctype(ClassItem::Xdigit { neg: false }))
            }
            b'H' if self.syntax.has_op2(op2::ESC_H_XDIGIT) => {
                Ok(class_ctype(ClassItem::Xdigit { neg: true }))
            }
            b'K' if self.syntax.has_op2(op2::ESC_CAPITAL_K_KEEP) => Ok(Node::Keep),
            b'R' if self.syntax.has_op2(op2::ESC_CAPITAL_R_GENERAL_NEWLINE) => Ok(Node::GeneralNewline),
            b'N' if self.syntax.has_op2(op2::ESC_CAPITAL_N_O_SUPER_DOT) => Ok(Node::Any { newline: false }),
            b'O' if self.syntax.has_op2(op2::ESC_CAPITAL_N_O_SUPER_DOT) => Ok(Node::SuperAny),
            b'X' if self.syntax.has_op2(op2::ESC_X_Y_TEXT_SEGMENT) => Ok(Node::TextSegment),
            b'y' if self.syntax.has_op2(op2::ESC_X_Y_TEXT_SEGMENT) => Ok(Node::Anchor(Anchor::TextSegBound)),
            b'Y' if self.syntax.has_op2(op2::ESC_X_Y_TEXT_SEGMENT) => {
                Ok(Node::Anchor(Anchor::NotTextSegBound))
            }
            b'`' if self.syntax.has_op2(op2::ESC_GNU_BUF_ANCHOR) => Ok(Node::Anchor(Anchor::Bos)),
            b'\'' if self.syntax.has_op2(op2::ESC_GNU_BUF_ANCHOR) => Ok(Node::Anchor(Anchor::Eos)),
            b'Q' if self.syntax.has_op2(op2::ESC_CAPITAL_Q_QUOTE) => self.parse_quote(),
            b'k' if self.syntax.has_op2(op2::ESC_K_NAMED_BACKREF) => self.parse_k_backref(),
            b'g' if self.syntax.has_op2(op2::ESC_G_SUBEXP_CALL) => self.parse_g_call(),
            b'p' | b'P' if self.syntax.has_op2(op2::ESC_P_BRACE_CHAR_PROPERTY) => {
                self.parse_prop(b == b'P')
            }
            b'x' => self.parse_hex_escape(),
            b'u' if self.syntax.has_op2(op2::ESC_U_HEX4) => self.parse_u_hex(4),
            b'o' if self.syntax.has_op(op::ESC_O_BRACE_OCTAL) => self.parse_oct_brace(),
            b'c' if self.syntax.has_op(op::ESC_C_CONTROL) => {
                let c = self.bump()?;
                Ok(Node::Char(c & 0x1f))
            }
            b'0'..=b'9' if self.syntax.has_op(op::DECIMAL_BACKREF) => {
                self.i -= 1;
                let n = self.read_int()? as i32;
                if n == 0 && self.syntax.has_op(op::ESC_OCTAL3) {
                    Ok(Node::Char(0))
                } else {
                    Ok(Node::Backref(Backref::Number(n)))
                }
            }
            _ if self.syntax.has_op(op::ESC_OCTAL3) && (b'0'..=b'7').contains(&b) => {
                self.i -= 1;
                Ok(Node::Char(self.read_octal()?))
            }
            _ => Ok(Node::Char(u32::from(b))),
        }
    }

    fn parse_quote(&mut self) -> Result<Node, Error> {
        let mut chars = Vec::new();
        loop {
            if self.peek_byte() == Some(b'\\') && self.peek_code_at(1) == Some(u32::from(b'E')) {
                self.i += 2;
                break;
            }
            if self.peek_code().is_none() {
                break;
            }
            chars.push(self.bump()?);
        }
        Ok(Node::Literal(chars))
    }

    fn parse_k_backref(&mut self) -> Result<Node, Error> {
        let q = self.peek_byte().ok_or_else(|| self.err("bad \\k"))?;
        if q != b'<' && q != b'\'' {
            return Err(self.err("bad \\k"));
        }
        self.i += 1;
        let end = if q == b'<' { b'>' } else { b'\'' };
        if self.peek_byte() == Some(b'-') || self.peek_byte() == Some(b'+') {
            let back = self.peek_byte() == Some(b'-');
            self.i += 1;
            let n = self.read_int()? as i32;
            if self.peek_byte() == Some(end) {
                self.i += 1;
            }
            return Ok(Node::Backref(Backref::Rel { back, n }));
        }
        if self.peek_byte().map(|b| b.is_ascii_digit()) == Some(true) {
            let n = self.read_int()? as i32;
            if self.peek_byte() == Some(end) {
                self.i += 1;
            }
            return Ok(Node::Backref(Backref::Number(n)));
        }
        let name = self.read_name(end)?;
        Ok(Node::Backref(Backref::Name(name)))
    }

    fn parse_g_call(&mut self) -> Result<Node, Error> {
        let q = self.peek_byte().ok_or_else(|| self.err("bad \\g"))?;
        if q != b'<' && q != b'\'' {
            return Err(self.err("bad \\g"));
        }
        self.i += 1;
        let end = if q == b'<' { b'>' } else { b'\'' };
        if self.peek_byte() == Some(b'0') {
            self.i += 1;
            if self.peek_byte() == Some(end) {
                self.i += 1;
            }
            return Ok(Node::Call(CallTarget::Whole));
        }
        if self.peek_byte() == Some(b'-') || self.peek_byte() == Some(b'+')
            || self.peek_byte().map(|b| b.is_ascii_digit()) == Some(true)
        {
            let n = self.read_signed_int()?;
            if self.peek_byte() == Some(end) {
                self.i += 1;
            }
            return Ok(Node::Call(CallTarget::Number(n)));
        }
        let name = self.read_name(end)?;
        Ok(Node::Call(CallTarget::Name(name)))
    }

    fn parse_prop(&mut self, cap_p: bool) -> Result<Node, Error> {
        let mut neg = cap_p;
        if self.peek_byte() == Some(b'{') {
            self.i += 1;
            if self.peek_byte() == Some(b'^') && self.syntax.has_op2(op2::ESC_P_BRACE_CIRCUMFLEX_NOT)
            {
                self.i += 1;
                neg = !neg;
            }
            let name = self.read_name(b'}')?;
            Ok(class_ctype(ClassItem::Prop { name, neg }))
        } else if self.syntax.has_behavior(behavior::ESC_P_WITH_ONE_CHAR_PROP) {
            let c = self.bump()?;
            let mut s = String::new();
            if let Some(ch) = char::from_u32(c) {
                s.push(ch);
            }
            Ok(class_ctype(ClassItem::Prop { name: s, neg }))
        } else {
            Err(self.err("bad \\p"))
        }
    }

    fn parse_hex_escape(&mut self) -> Result<Node, Error> {
        if self.peek_byte() == Some(b'{') && self.syntax.has_op(op::ESC_X_BRACE_HEX8) {
            self.i += 1;
            let mut v = 0u32;
            while let Some(h) = self.peek_byte().and_then(hex_val) {
                self.i += 1;
                v = (v << 4) | u32::from(h);
            }
            if self.peek_byte() == Some(b'}') {
                self.i += 1;
            }
            return Ok(Node::Char(v));
        }
        if self.syntax.has_op(op::ESC_X_HEX2) {
            let mut v = 0u32;
            for _ in 0..2 {
                if let Some(h) = self.peek_byte().and_then(hex_val) {
                    self.i += 1;
                    v = (v << 4) | u32::from(h);
                }
            }
            return Ok(Node::Char(v));
        }
        Ok(Node::Char(b'x' as u32))
    }

    fn parse_u_hex(&mut self, n: usize) -> Result<Node, Error> {
        let mut v = 0u32;
        for _ in 0..n {
            let h = self.peek_byte().and_then(hex_val).ok_or_else(|| self.err("bad \\u"))?;
            self.i += 1;
            v = (v << 4) | u32::from(h);
        }
        Ok(Node::Char(v))
    }

    fn parse_oct_brace(&mut self) -> Result<Node, Error> {
        if self.peek_byte() != Some(b'{') {
            return Ok(Node::Char(b'o' as u32));
        }
        self.i += 1;
        let mut v = 0u32;
        while let Some(b) = self.peek_byte() {
            if (b'0'..=b'7').contains(&b) {
                self.i += 1;
                v = (v << 3) | u32::from(b - b'0');
            } else {
                break;
            }
        }
        if self.peek_byte() == Some(b'}') {
            self.i += 1;
        }
        Ok(Node::Char(v))
    }

    fn parse_class(&mut self) -> Result<Node, Error> {
        self.i += 1;
        let mut cc = CharClass::empty();
        if self.peek_byte() == Some(b'^') {
            cc.negate = true;
            self.i += 1;
        }
        cc.items = self.parse_class_items(b']')?;
        if self.peek_byte() != Some(b']') {
            return Err(self.err("unmatched ["));
        }
        self.i += 1;
        Ok(Node::Class(cc))
    }

    fn parse_class_items(&mut self, end: u8) -> Result<Vec<ClassItem>, Error> {
        // Nested classes recurse here rather than through `parse_alts`, so
        // they need the same bound.
        self.depth += 1;
        self.check_depth()?;
        let r = self.parse_class_items_inner(end);
        self.depth -= 1;
        r
    }

    fn parse_class_items_inner(&mut self, end: u8) -> Result<Vec<ClassItem>, Error> {
        let mut items = Vec::new();
        let mut first = true;
        while self.peek_code().is_some() && (first || self.peek_byte() != Some(end)) {
            first = false;
            if self.peek_byte() == Some(b'[') && self.syntax.has_op2(op2::CCLASS_SET_OP) {
                if self.peek_code_at(1) == Some(u32::from(b':')) && self.syntax.has_op(op::POSIX_BRACKET)
                {
                    items.push(self.parse_posix()?);
                    continue;
                }
                self.i += 1;
                let mut nested = CharClass::empty();
                if self.peek_byte() == Some(b'^') {
                    nested.negate = true;
                    self.i += 1;
                }
                nested.items = self.parse_class_items(b']')?;
                if self.peek_byte() == Some(b']') {
                    self.i += 1;
                }
                items.push(ClassItem::Nested(nested));
                continue;
            }
            if self.peek_byte() == Some(b'&')
                && self.peek_code_at(1) == Some(u32::from(b'&'))
                && self.syntax.has_op2(op2::CCLASS_SET_OP)
            {
                self.i += 2;
                let rest = self.parse_class_items(end)?;
                items.push(ClassItem::Intersect(rest));
                break;
            }
            if self.peek_byte() == Some(b'\\') {
                self.i += 1;
                match self.peek_byte() {
                    Some(b'w') => {
                        self.i += 1;
                        items.push(ClassItem::Word { neg: false });
                    }
                    Some(b'W') => {
                        self.i += 1;
                        items.push(ClassItem::Word { neg: true });
                    }
                    Some(b'd') => {
                        self.i += 1;
                        items.push(ClassItem::Digit { neg: false });
                    }
                    Some(b'D') => {
                        self.i += 1;
                        items.push(ClassItem::Digit { neg: true });
                    }
                    Some(b's') => {
                        self.i += 1;
                        items.push(ClassItem::Space { neg: false });
                    }
                    Some(b'S') => {
                        self.i += 1;
                        items.push(ClassItem::Space { neg: true });
                    }
                    Some(b'h') => {
                        self.i += 1;
                        items.push(ClassItem::Xdigit { neg: false });
                    }
                    Some(b'H') => {
                        self.i += 1;
                        items.push(ClassItem::Xdigit { neg: true });
                    }
                    Some(b'n') => {
                        self.i += 1;
                        items.push(ClassItem::Char(0x0a));
                    }
                    Some(b't') => {
                        self.i += 1;
                        items.push(ClassItem::Char(0x09));
                    }
                    // \p{...} / \P{...} are class items in their own right.
                    Some(b'p') | Some(b'P')
                        if self.syntax.has_op2(op2::ESC_P_BRACE_CHAR_PROPERTY) =>
                    {
                        let cap_p = self.peek_byte() == Some(b'P');
                        self.i += 1;
                        if let Node::Class(cc) = self.parse_prop(cap_p)? {
                            items.extend(cc.items);
                        } else {
                            return Err(self.err("bad property in class"));
                        }
                    }
                    Some(_) => {
                        // Everything else denoting a single codepoint -- \xHH,
                        // \x{...}, \uHHHH, \o{...}, \cX, octal, \r \f \v \a \e
                        // -- goes through the same readers used outside a
                        // class. Leaving them to the literal fallback made
                        // `[\x{00e9}]` a class of `0`, `e` and `9`.
                        let c = self.class_escape_char()?;
                        if self.peek_byte() == Some(b'-')
                            && self.peek_code_at(1) != Some(u32::from(end))
                            && self.peek_code_at(1).is_some()
                        {
                            self.i += 1;
                            let d = if self.peek_byte() == Some(b'\\') {
                                self.i += 1;
                                self.class_escape_char()?
                            } else {
                                self.bump()?
                            };
                            items.push(ClassItem::Range(c, d));
                        } else {
                            items.push(ClassItem::Char(c));
                        }
                    }
                    None => return Err(self.err("end in class")),
                }
                continue;
            }
            let c = self.bump()?;
            if self.peek_byte() == Some(b'-') && self.peek_code_at(1) != Some(u32::from(end)) {
                self.i += 1;
                let d = if self.peek_byte() == Some(b'\\') {
                    self.i += 1;
                    self.class_escape_char()?
                } else {
                    self.bump()?
                };
                items.push(ClassItem::Range(c, d));
            } else {
                items.push(ClassItem::Char(c));
            }
        }
        Ok(items)
    }

    /// Read one codepoint-valued escape inside a character class.
    ///
    /// Shares the readers with [`Self::parse_escape`] so a class and a bare
    /// pattern cannot disagree about what `\x{...}` means. `self.i` is already
    /// past the backslash.
    fn class_escape_char(&mut self) -> Result<u32, Error> {
        let b = match self.peek_byte() {
            Some(b) => b,
            None => return Err(self.err("end in class")),
        };
        let node = match b {
            b'x' => {
                self.i += 1;
                self.parse_hex_escape()?
            }
            b'u' if self.syntax.has_op2(op2::ESC_U_HEX4) => {
                self.i += 1;
                self.parse_u_hex(4)?
            }
            b'o' if self.syntax.has_op(op::ESC_O_BRACE_OCTAL) => {
                self.i += 1;
                self.parse_oct_brace()?
            }
            b'c' if self.syntax.has_op(op::ESC_C_CONTROL) => {
                self.i += 1;
                let c = self.bump()?;
                Node::Char(c & 0x1f)
            }
            b'r' => {
                self.i += 1;
                Node::Char(0x0d)
            }
            b'f' => {
                self.i += 1;
                Node::Char(0x0c)
            }
            b'v' => {
                self.i += 1;
                Node::Char(0x0b)
            }
            b'a' => {
                self.i += 1;
                Node::Char(0x07)
            }
            b'e' => {
                self.i += 1;
                Node::Char(0x1b)
            }
            // Inside a class `\b` is a backspace, not a word boundary.
            b'b' => {
                self.i += 1;
                Node::Char(0x08)
            }
            b'0'..=b'7' if self.syntax.has_op(op::ESC_OCTAL3) => Node::Char(self.read_octal()?),
            _ => Node::Char(self.bump()?),
        };
        match node {
            Node::Char(c) => Ok(c),
            _ => Err(self.err("escape is not a single character in class")),
        }
    }

    fn parse_posix(&mut self) -> Result<ClassItem, Error> {
        self.i += 2;
        let neg = if self.peek_byte() == Some(b'^') {
            self.i += 1;
            true
        } else {
            false
        };
        let name = self.collect_until(b':');
        if self.peek_byte() == Some(b':') {
            self.i += 1;
        }
        if self.peek_byte() == Some(b']') {
            self.i += 1;
        }
        Ok(ClassItem::Posix { name, neg })
    }

    fn expect_close(&mut self) -> Result<(), Error> {
        if self.peek_byte() != Some(b')') {
            return Err(self.err("unmatched parenthesis"));
        }
        self.i += 1;
        Ok(())
    }

    fn read_name(&mut self, end: u8) -> Result<String, Error> {
        let name = self.collect_until(end);
        if name.is_empty() {
            return Err(self.err("empty group name"));
        }
        if self.peek_byte() == Some(end) {
            self.i += 1;
        }
        Ok(name)
    }

    fn read_ident(&mut self) -> Result<String, Error> {
        let mut name = String::new();
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_alphanumeric() || b == b'_' {
                name.push(b as char);
                self.i += 1;
            } else {
                break;
            }
        }
        Ok(name)
    }

    fn read_ascii_digits(&mut self) -> String {
        let mut s = String::new();
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_digit() {
                s.push(b as char);
                self.i += 1;
            } else {
                break;
            }
        }
        s
    }

    fn collect_until(&mut self, end: u8) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek_code() {
            if c == u32::from(end) {
                break;
            }
            if let Some(ch) = char::from_u32(c) {
                s.push(ch);
            }
            self.i += 1;
        }
        s
    }

    fn read_int(&mut self) -> Result<u32, Error> {
        let s = self.read_ascii_digits();
        if s.is_empty() {
            return Err(self.err("expected number"));
        }
        s.parse().map_err(|_| self.err("bad number"))
    }

    fn read_signed_int(&mut self) -> Result<i32, Error> {
        let neg = if self.peek_byte() == Some(b'-') {
            self.i += 1;
            true
        } else if self.peek_byte() == Some(b'+') {
            self.i += 1;
            false
        } else {
            false
        };
        let n = self.read_int()? as i32;
        Ok(if neg { -n } else { n })
    }

    fn read_octal(&mut self) -> Result<u32, Error> {
        let mut v = 0u32;
        for _ in 0..3 {
            match self.peek_byte() {
                Some(b) if (b'0'..=b'7').contains(&b) => {
                    self.i += 1;
                    v = (v << 3) | u32::from(b - b'0');
                }
                _ => break,
            }
        }
        Ok(v)
    }
}

fn check_never_ending_recursion(root: &Node, names: &[Option<String>]) -> Result<(), Error> {
    fn groups<'a>(n: &'a Node, out: &mut Vec<(usize, &'a Node)>) {
        match n {
            Node::Capture { index, inner, .. } => {
                out.push((*index, inner));
                groups(inner, out);
            }
            Node::Concat(v) | Node::Alt(v) => {
                for x in v {
                    groups(x, out);
                }
            }
            Node::Repeat { inner, .. }
            | Node::Group(inner)
            | Node::Look { inner, .. }
            | Node::Atomic(inner)
            | Node::Options { inner, .. } => groups(inner, out),
            Node::Absent { stopper, expr, .. } => {
                groups(stopper, out);
                if let Some(e) = expr {
                    groups(e, out);
                }
            }
            Node::Conditional {
                then_n, else_n, ..
            } => {
                groups(then_n, out);
                if let Some(e) = else_n {
                    groups(e, out);
                }
            }
            _ => {}
        }
    }
    let mut gs = Vec::new();
    groups(root, &mut gs);
    for (idx, inner) in &gs {
        if left_calls(inner, *idx, names) {
            return Err(Error::kind_msg(
                ErrorKind::NeverEndingRecursion,
                "never-ending recursion",
            ));
        }
    }
    Ok(())
}

fn left_calls(n: &Node, target: usize, names: &[Option<String>]) -> bool {
    match n {
        Node::Empty | Node::Anchor(_) | Node::Keep | Node::Look { .. } => false,
        Node::Call(t) => call_is(t, target, names),
        Node::Concat(v) => {
            for x in v {
                if nullable(x) {
                    if left_calls(x, target, names) {
                        return true;
                    }
                    continue;
                }
                return left_calls(x, target, names);
            }
            false
        }
        Node::Alt(v) => v.iter().any(|x| left_calls(x, target, names)),
        Node::Repeat { inner, .. }
        | Node::Group(inner)
        | Node::Capture { inner, .. }
        | Node::Atomic(inner)
        | Node::Options { inner, .. } => left_calls(inner, target, names),
        Node::Conditional {
            then_n, else_n, ..
        } => {
            left_calls(then_n, target, names)
                || else_n.as_ref().map(|e| left_calls(e, target, names)).unwrap_or(false)
        }
        _ => false,
    }
}

fn call_is(t: &CallTarget, target: usize, names: &[Option<String>]) -> bool {
    match t {
        CallTarget::Number(n) if *n >= 0 && *n as usize == target => true,
        CallTarget::Whole if target == 0 => true,
        CallTarget::Name(n) => names.get(target).and_then(|s| s.as_deref()) == Some(n.as_str()),
        _ => false,
    }
}

fn nullable(n: &Node) -> bool {
    match n {
        Node::Empty | Node::Anchor(_) | Node::Keep | Node::Look { .. } => true,
        Node::Repeat { min, .. } => *min == 0,
        Node::Group(inner) | Node::Capture { inner, .. } | Node::Options { inner, .. } => {
            nullable(inner)
        }
        Node::Concat(v) => v.iter().all(nullable),
        Node::Alt(v) => v.iter().any(nullable),
        _ => false,
    }
}

fn class_ctype(item: ClassItem) -> Node {
    Node::Class(CharClass {
        negate: false,
        items: {
            let mut v = Vec::new();
            v.push(item);
            v
        },
    })
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[allow(dead_code)]
fn _syntax_mod_used() {
    let _ = syntax::op::DOT_ANYCHAR;
}
