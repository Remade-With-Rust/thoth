//! Regex AST.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::syntax::Options;

#[derive(Clone, Debug)]
pub enum Node {
    Empty,
    Char(u32),
    Literal(Vec<u32>),
    Any { newline: bool },
    /// `\O` true anychar.
    SuperAny,
    Class(CharClass),
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat {
        inner: Box<Node>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        possessive: bool,
    },
    Capture {
        index: usize,
        name: Option<String>,
        inner: Box<Node>,
        history: bool,
    },
    Group(Box<Node>),
    Anchor(Anchor),
    Backref(Backref),
    Look {
        behind: bool,
        negative: bool,
        inner: Box<Node>,
    },
    Atomic(Box<Node>),
    Call(CallTarget),
    Absent {
        stopper: Box<Node>,
        expr: Option<Box<Node>>,
        kind: AbsentKind,
    },
    Conditional {
        cond: Cond,
        then_n: Box<Node>,
        else_n: Option<Box<Node>>,
    },
    Keep,
    /// `\R` general newline.
    GeneralNewline,
    /// `\X` text segment.
    TextSegment,
    Callout {
        named: bool,
        name: String,
        args: String,
        tag: Option<String>,
        body: String,
        dir: super::callout::CalloutDir,
    },
    /// Isolated option applying to the rest of the current group: stored as wrapper.
    Options {
        set: Options,
        clear: Options,
        inner: Box<Node>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbsentKind {
    Repeater,
    Expression,
    Stopper,
    Clear,
}

#[derive(Clone, Debug)]
pub enum Cond {
    Group(usize),
    Name(String),
    Expr(Box<Node>),
    ValidRef(Backref),
    /// Compiled look-around used as a conditional test (does not consume).
    Look { body: u16, after: u16 },
}

#[derive(Clone, Debug)]
pub enum CallTarget {
    Number(i32),
    Name(String),
    Whole,
}

#[derive(Clone, Debug)]
pub enum Backref {
    Number(i32),
    Name(String),
    Rel { back: bool, n: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Bol,
    Eol,
    Bos,
    Eos,
    EosNl,
    WordBound,
    NotWordBound,
    WordBegin,
    WordEnd,
    G,
    TextSegBound,
    NotTextSegBound,
}

#[derive(Clone, Debug)]
pub struct CharClass {
    pub negate: bool,
    pub items: Vec<ClassItem>,
}

#[derive(Clone, Debug)]
pub enum ClassItem {
    Char(u32),
    Range(u32, u32),
    Posix { name: String, neg: bool },
    Prop { name: String, neg: bool },
    Word { neg: bool },
    Digit { neg: bool },
    Space { neg: bool },
    Xdigit { neg: bool },
    Nested(CharClass),
    Intersect(Vec<ClassItem>),
}

impl CharClass {
    pub fn empty() -> Self {
        Self {
            negate: false,
            items: Vec::new(),
        }
    }
}

pub fn concat(mut v: Vec<Node>) -> Node {
    match v.len() {
        0 => Node::Empty,
        1 => v.pop().unwrap(),
        _ => Node::Concat(v),
    }
}

pub fn alt(mut v: Vec<Node>) -> Node {
    match v.len() {
        0 => Node::Empty,
        1 => v.pop().unwrap(),
        _ => Node::Alt(v),
    }
}
