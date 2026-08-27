//! Compile/search options and `OnigSyntaxType` flag tables.

use super::encoding::Encoding;

/// Compile- and search-time option bits (Oniguruma `OnigOptionType`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options(pub u32);

impl Options {
    pub const NONE: Self = Self(0);
    pub const IGNORECASE: Self = Self(1);
    pub const EXTEND: Self = Self(1 << 1);
    pub const MULTILINE: Self = Self(1 << 2);
    pub const SINGLELINE: Self = Self(1 << 3);
    pub const FIND_LONGEST: Self = Self(1 << 4);
    pub const FIND_NOT_EMPTY: Self = Self(1 << 5);
    pub const NEGATE_SINGLELINE: Self = Self(1 << 6);
    pub const DONT_CAPTURE_GROUP: Self = Self(1 << 7);
    pub const CAPTURE_GROUP: Self = Self(1 << 8);
    pub const NOTBOL: Self = Self(1 << 9);
    pub const NOTEOL: Self = Self(1 << 10);
    pub const IGNORECASE_IS_ASCII: Self = Self(1 << 13);
    pub const WORD_IS_ASCII: Self = Self(1 << 14);
    pub const DIGIT_IS_ASCII: Self = Self(1 << 15);
    pub const SPACE_IS_ASCII: Self = Self(1 << 16);
    pub const POSIX_IS_ASCII: Self = Self(1 << 17);
    pub const TEXT_SEGMENT_EXTENDED_GRAPHEME_CLUSTER: Self = Self(1 << 18);
    pub const TEXT_SEGMENT_WORD: Self = Self(1 << 19);
    pub const NOT_BEGIN_STRING: Self = Self(1 << 20);
    pub const NOT_END_STRING: Self = Self(1 << 21);
    pub const NOT_BEGIN_POSITION: Self = Self(1 << 22);
    pub const CALLBACK_EACH_MATCH: Self = Self(1 << 23);
    pub const MATCH_WHOLE_STRING: Self = Self(1 << 24);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub fn set(&mut self, other: Self, on: bool) {
        if on {
            self.0 |= other.0;
        } else {
            self.0 &= !other.0;
        }
    }
}

impl core::ops::BitOr for Options {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// Syntax operator/behavior bits (`OnigSyntaxType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Syntax {
    pub op: u32,
    pub op2: u32,
    pub behavior: u32,
    pub options: Options,
    pub meta_escape: u32,
    pub meta_anychar: u32,
    pub meta_anytime: u32,
    pub meta_zero_or_one: u32,
    pub meta_one_or_more: u32,
    pub meta_anychar_anytime: u32,
}

pub mod op {
    pub const VARIABLE_META_CHARACTERS: u32 = 1 << 0;
    pub const DOT_ANYCHAR: u32 = 1 << 1;
    pub const ASTERISK_ZERO_INF: u32 = 1 << 2;
    pub const ESC_ASTERISK_ZERO_INF: u32 = 1 << 3;
    pub const PLUS_ONE_INF: u32 = 1 << 4;
    pub const ESC_PLUS_ONE_INF: u32 = 1 << 5;
    pub const QMARK_ZERO_ONE: u32 = 1 << 6;
    pub const ESC_QMARK_ZERO_ONE: u32 = 1 << 7;
    pub const BRACE_INTERVAL: u32 = 1 << 8;
    pub const ESC_BRACE_INTERVAL: u32 = 1 << 9;
    pub const VBAR_ALT: u32 = 1 << 10;
    pub const ESC_VBAR_ALT: u32 = 1 << 11;
    pub const LPAREN_SUBEXP: u32 = 1 << 12;
    pub const ESC_LPAREN_SUBEXP: u32 = 1 << 13;
    pub const ESC_AZ_BUF_ANCHOR: u32 = 1 << 14;
    pub const ESC_CAPITAL_G_BEGIN_ANCHOR: u32 = 1 << 15;
    pub const DECIMAL_BACKREF: u32 = 1 << 16;
    pub const BRACKET_CC: u32 = 1 << 17;
    pub const ESC_W_WORD: u32 = 1 << 18;
    pub const ESC_LTGT_WORD_BEGIN_END: u32 = 1 << 19;
    pub const ESC_B_WORD_BOUND: u32 = 1 << 20;
    pub const ESC_S_WHITE_SPACE: u32 = 1 << 21;
    pub const ESC_D_DIGIT: u32 = 1 << 22;
    pub const LINE_ANCHOR: u32 = 1 << 23;
    pub const POSIX_BRACKET: u32 = 1 << 24;
    pub const QMARK_NON_GREEDY: u32 = 1 << 25;
    pub const ESC_CONTROL_CHARS: u32 = 1 << 26;
    pub const ESC_C_CONTROL: u32 = 1 << 27;
    pub const ESC_OCTAL3: u32 = 1 << 28;
    pub const ESC_X_HEX2: u32 = 1 << 29;
    pub const ESC_X_BRACE_HEX8: u32 = 1 << 30;
    pub const ESC_O_BRACE_OCTAL: u32 = 1 << 31;
}

pub mod op2 {
    pub const ESC_CAPITAL_Q_QUOTE: u32 = 1 << 0;
    pub const QMARK_GROUP_EFFECT: u32 = 1 << 1;
    pub const OPTION_PERL: u32 = 1 << 2;
    pub const OPTION_RUBY: u32 = 1 << 3;
    pub const PLUS_POSSESSIVE_REPEAT: u32 = 1 << 4;
    pub const PLUS_POSSESSIVE_INTERVAL: u32 = 1 << 5;
    pub const CCLASS_SET_OP: u32 = 1 << 6;
    pub const QMARK_LT_NAMED_GROUP: u32 = 1 << 7;
    pub const ESC_K_NAMED_BACKREF: u32 = 1 << 8;
    pub const ESC_G_SUBEXP_CALL: u32 = 1 << 9;
    pub const ESC_CAPITAL_C_BAR_CONTROL: u32 = 1 << 11;
    pub const ESC_CAPITAL_M_BAR_META: u32 = 1 << 12;
    pub const ESC_V_VTAB: u32 = 1 << 13;
    pub const ESC_U_HEX4: u32 = 1 << 14;
    pub const ESC_GNU_BUF_ANCHOR: u32 = 1 << 15;
    pub const ESC_P_BRACE_CHAR_PROPERTY: u32 = 1 << 16;
    pub const ESC_P_BRACE_CIRCUMFLEX_NOT: u32 = 1 << 17;
    pub const ESC_H_XDIGIT: u32 = 1 << 19;
    pub const INEFFECTIVE_ESCAPE: u32 = 1 << 20;
    pub const QMARK_LPAREN_IF_ELSE: u32 = 1 << 21;
    pub const ESC_CAPITAL_K_KEEP: u32 = 1 << 22;
    pub const ESC_CAPITAL_R_GENERAL_NEWLINE: u32 = 1 << 23;
    pub const ESC_CAPITAL_N_O_SUPER_DOT: u32 = 1 << 24;
    pub const QMARK_TILDE_ABSENT_GROUP: u32 = 1 << 25;
    pub const ESC_X_Y_TEXT_SEGMENT: u32 = 1 << 26;
    pub const QMARK_PERL_SUBEXP_CALL: u32 = 1 << 27;
    pub const QMARK_BRACE_CALLOUT_CONTENTS: u32 = 1 << 28;
    pub const ASTERISK_CALLOUT_NAME: u32 = 1 << 29;
    pub const OPTION_ONIGURUMA: u32 = 1 << 30;
    pub const QMARK_CAPITAL_P_NAME: u32 = 1 << 31;
}

pub mod behavior {
    pub const CONTEXT_INDEP_REPEAT_OPS: u32 = 1 << 0;
    pub const CONTEXT_INVALID_REPEAT_OPS: u32 = 1 << 1;
    pub const ALLOW_UNMATCHED_CLOSE_SUBEXP: u32 = 1 << 2;
    pub const ALLOW_INVALID_INTERVAL: u32 = 1 << 3;
    pub const ALLOW_INTERVAL_LOW_ABBREV: u32 = 1 << 4;
    pub const STRICT_CHECK_BACKREF: u32 = 1 << 5;
    pub const DIFFERENT_LEN_ALT_LOOK_BEHIND: u32 = 1 << 6;
    pub const CAPTURE_ONLY_NAMED_GROUP: u32 = 1 << 7;
    pub const ALLOW_MULTIPLEX_DEFINITION_NAME: u32 = 1 << 8;
    pub const FIXED_INTERVAL_IS_GREEDY_ONLY: u32 = 1 << 9;
    pub const ISOLATED_OPTION_CONTINUE_BRANCH: u32 = 1 << 10;
    pub const VARIABLE_LEN_LOOK_BEHIND: u32 = 1 << 11;
    pub const PYTHON: u32 = 1 << 12;
    pub const WHOLE_OPTIONS: u32 = 1 << 13;
    pub const BRE_ANCHOR_AT_EDGE_OF_SUBEXP: u32 = 1 << 14;
    pub const ESC_P_WITH_ONE_CHAR_PROP: u32 = 1 << 15;
    pub const NOT_NEWLINE_IN_NEGATIVE_CC: u32 = 1 << 20;
    pub const BACKSLASH_ESCAPE_IN_CC: u32 = 1 << 21;
    pub const ALLOW_EMPTY_RANGE_IN_CC: u32 = 1 << 22;
    pub const ALLOW_DOUBLE_RANGE_OP_IN_CC: u32 = 1 << 23;
    pub const ALLOW_CHAR_TYPE_FOLLOWED_BY_MINUS_IN_CC: u32 = 1 << 27;
    /// Oniguruma names this `CONTEXT_INDEP_ANCHORS` at bit 31; unused in practice.
    pub const CONTEXT_INDEP_ANCHORS_DUMMY: u32 = 0;
}

const GNU_OP: u32 = op::DOT_ANYCHAR
    | op::BRACKET_CC
    | op::POSIX_BRACKET
    | op::ASTERISK_ZERO_INF
    | op::PLUS_ONE_INF
    | op::QMARK_ZERO_ONE
    | op::BRACE_INTERVAL
    | op::VBAR_ALT
    | op::LPAREN_SUBEXP
    | op::ESC_AZ_BUF_ANCHOR
    | op::ESC_CAPITAL_G_BEGIN_ANCHOR
    | op::DECIMAL_BACKREF
    | op::ESC_W_WORD
    | op::ESC_B_WORD_BOUND
    | op::ESC_S_WHITE_SPACE
    | op::ESC_D_DIGIT
    | op::LINE_ANCHOR
    | op::ESC_LTGT_WORD_BEGIN_END;

const GNU_BV: u32 = behavior::CONTEXT_INDEP_ANCHORS_DUMMY
    | behavior::CONTEXT_INDEP_REPEAT_OPS
    | behavior::CONTEXT_INVALID_REPEAT_OPS
    | behavior::ALLOW_INVALID_INTERVAL
    | behavior::BACKSLASH_ESCAPE_IN_CC
    | behavior::ALLOW_DOUBLE_RANGE_OP_IN_CC;

fn meta_default() -> (u32, u32, u32, u32, u32, u32) {
    (b'\\' as u32, 0, 0, 0, 0, 0)
}

impl Syntax {
    fn with(op: u32, op2: u32, behavior: u32, options: Options) -> Self {
        let m = meta_default();
        Self {
            op,
            op2,
            behavior,
            options,
            meta_escape: m.0,
            meta_anychar: m.1,
            meta_anytime: m.2,
            meta_zero_or_one: m.3,
            meta_one_or_more: m.4,
            meta_anychar_anytime: m.5,
        }
    }

    pub fn has_op(self, bit: u32) -> bool {
        self.op & bit != 0
    }

    pub fn has_op2(self, bit: u32) -> bool {
        self.op2 & bit != 0
    }

    pub fn has_behavior(self, bit: u32) -> bool {
        self.behavior & bit != 0
    }

    /// Set a variable meta character (`onig_set_meta_char`).
    pub fn set_meta_char(&mut self, what: u32, code: u32) {
        match what {
            0 => self.meta_escape = code,
            1 => self.meta_anychar = code,
            2 => self.meta_anytime = code,
            3 => self.meta_zero_or_one = code,
            4 => self.meta_one_or_more = code,
            5 => self.meta_anychar_anytime = code,
            _ => {}
        }
    }

    pub const ASIS: Self = Self {
        op: 0,
        op2: op2::INEFFECTIVE_ESCAPE,
        behavior: 0,
        options: Options::NONE,
        meta_escape: b'\\' as u32,
        meta_anychar: 0,
        meta_anytime: 0,
        meta_zero_or_one: 0,
        meta_one_or_more: 0,
        meta_anychar_anytime: 0,
    };

    pub fn posix_basic() -> Self {
        Self::with(
            GNU_OP & !(op::PLUS_ONE_INF | op::QMARK_ZERO_ONE | op::VBAR_ALT | op::LPAREN_SUBEXP)
                | op::ESC_LPAREN_SUBEXP
                | op::ESC_BRACE_INTERVAL
                | op::POSIX_BRACKET
                | op::DOT_ANYCHAR
                | op::BRACKET_CC
                | op::ASTERISK_ZERO_INF
                | op::LINE_ANCHOR,
            0,
            behavior::BRE_ANCHOR_AT_EDGE_OF_SUBEXP,
            Options::SINGLELINE.union(Options::MULTILINE),
        )
    }

    pub fn posix_extended() -> Self {
        Self::with(
            GNU_OP,
            0,
            behavior::CONTEXT_INDEP_REPEAT_OPS
                | behavior::CONTEXT_INVALID_REPEAT_OPS
                | behavior::ALLOW_UNMATCHED_CLOSE_SUBEXP
                | behavior::ALLOW_DOUBLE_RANGE_OP_IN_CC,
            Options::SINGLELINE.union(Options::MULTILINE),
        )
    }

    pub fn emacs() -> Self {
        Self::with(
            op::DOT_ANYCHAR
                | op::BRACKET_CC
                | op::ESC_BRACE_INTERVAL
                | op::ESC_LPAREN_SUBEXP
                | op::ESC_VBAR_ALT
                | op::ASTERISK_ZERO_INF
                | op::PLUS_ONE_INF
                | op::QMARK_ZERO_ONE
                | op::DECIMAL_BACKREF
                | op::LINE_ANCHOR
                | op::ESC_CONTROL_CHARS,
            op2::ESC_GNU_BUF_ANCHOR | op2::QMARK_GROUP_EFFECT,
            0,
            Options::NONE,
        )
    }

    pub fn grep() -> Self {
        Self::with(
            op::DOT_ANYCHAR
                | op::BRACKET_CC
                | op::POSIX_BRACKET
                | op::ESC_BRACE_INTERVAL
                | op::ESC_LPAREN_SUBEXP
                | op::ESC_VBAR_ALT
                | op::ASTERISK_ZERO_INF
                | op::ESC_PLUS_ONE_INF
                | op::ESC_QMARK_ZERO_ONE
                | op::LINE_ANCHOR
                | op::ESC_W_WORD
                | op::ESC_B_WORD_BOUND
                | op::ESC_LTGT_WORD_BEGIN_END
                | op::DECIMAL_BACKREF,
            0,
            behavior::NOT_NEWLINE_IN_NEGATIVE_CC | behavior::BRE_ANCHOR_AT_EDGE_OF_SUBEXP,
            Options::NONE,
        )
    }

    pub fn gnu_regex() -> Self {
        Self::with(GNU_OP, 0, GNU_BV, Options::NONE)
    }

    pub fn java() -> Self {
        Self::with(
            (GNU_OP | op::QMARK_NON_GREEDY | op::ESC_CONTROL_CHARS | op::ESC_C_CONTROL | op::ESC_OCTAL3 | op::ESC_X_HEX2)
                & !(op::ESC_LTGT_WORD_BEGIN_END | op::POSIX_BRACKET),
            op2::ESC_CAPITAL_Q_QUOTE
                | op2::QMARK_GROUP_EFFECT
                | op2::OPTION_PERL
                | op2::PLUS_POSSESSIVE_REPEAT
                | op2::PLUS_POSSESSIVE_INTERVAL
                | op2::CCLASS_SET_OP
                | op2::ESC_V_VTAB
                | op2::ESC_U_HEX4
                | op2::ESC_P_BRACE_CHAR_PROPERTY,
            GNU_BV
                | behavior::ISOLATED_OPTION_CONTINUE_BRANCH
                | behavior::DIFFERENT_LEN_ALT_LOOK_BEHIND
                | behavior::VARIABLE_LEN_LOOK_BEHIND
                | behavior::ALLOW_CHAR_TYPE_FOLLOWED_BY_MINUS_IN_CC,
            Options::SINGLELINE,
        )
    }

    pub fn perl() -> Self {
        Self::with(
            (GNU_OP
                | op::QMARK_NON_GREEDY
                | op::ESC_OCTAL3
                | op::ESC_X_HEX2
                | op::ESC_X_BRACE_HEX8
                | op::ESC_O_BRACE_OCTAL
                | op::ESC_CONTROL_CHARS
                | op::ESC_C_CONTROL)
                & !op::ESC_LTGT_WORD_BEGIN_END,
            op2::ESC_CAPITAL_Q_QUOTE
                | op2::QMARK_GROUP_EFFECT
                | op2::OPTION_PERL
                | op2::PLUS_POSSESSIVE_REPEAT
                | op2::PLUS_POSSESSIVE_INTERVAL
                | op2::QMARK_LPAREN_IF_ELSE
                | op2::QMARK_TILDE_ABSENT_GROUP
                | op2::QMARK_BRACE_CALLOUT_CONTENTS
                | op2::ASTERISK_CALLOUT_NAME
                | op2::ESC_X_Y_TEXT_SEGMENT
                | op2::ESC_P_BRACE_CHAR_PROPERTY
                | op2::ESC_P_BRACE_CIRCUMFLEX_NOT
                | op2::ESC_CAPITAL_K_KEEP
                | op2::ESC_CAPITAL_R_GENERAL_NEWLINE
                | op2::ESC_CAPITAL_N_O_SUPER_DOT,
            GNU_BV
                | behavior::ISOLATED_OPTION_CONTINUE_BRANCH
                | behavior::ALLOW_CHAR_TYPE_FOLLOWED_BY_MINUS_IN_CC
                | behavior::ESC_P_WITH_ONE_CHAR_PROP,
            Options::SINGLELINE,
        )
    }

    pub fn perl_ng() -> Self {
        let mut s = Self::perl();
        s.op2 |= op2::QMARK_LT_NAMED_GROUP
            | op2::ESC_K_NAMED_BACKREF
            | op2::ESC_G_SUBEXP_CALL
            | op2::QMARK_PERL_SUBEXP_CALL;
        s.behavior |= behavior::CAPTURE_ONLY_NAMED_GROUP | behavior::ALLOW_MULTIPLEX_DEFINITION_NAME;
        s
    }

    pub fn python() -> Self {
        Self::with(
            (GNU_OP
                | op::QMARK_NON_GREEDY
                | op::ESC_OCTAL3
                | op::ESC_X_HEX2
                | op::ESC_CONTROL_CHARS
                | op::ESC_C_CONTROL)
                & !(op::ESC_LTGT_WORD_BEGIN_END | op::POSIX_BRACKET),
            op2::QMARK_GROUP_EFFECT
                | op2::OPTION_PERL
                | op2::QMARK_LPAREN_IF_ELSE
                | op2::ASTERISK_CALLOUT_NAME
                | op2::ESC_P_BRACE_CHAR_PROPERTY
                | op2::ESC_P_BRACE_CIRCUMFLEX_NOT
                | op2::QMARK_CAPITAL_P_NAME
                | op2::ESC_CAPITAL_K_KEEP
                | op2::ESC_V_VTAB
                | op2::ESC_U_HEX4,
            GNU_BV
                | behavior::ISOLATED_OPTION_CONTINUE_BRANCH
                | behavior::ALLOW_INTERVAL_LOW_ABBREV
                | behavior::PYTHON,
            Options::SINGLELINE,
        )
    }

    /// Default syntax (`ONIG_SYNTAX_ONIGURUMA`).
    pub const fn oniguruma() -> Self {
        Self {
            op: (GNU_OP
                | op::QMARK_NON_GREEDY
                | op::ESC_OCTAL3
                | op::ESC_X_HEX2
                | op::ESC_X_BRACE_HEX8
                | op::ESC_O_BRACE_OCTAL
                | op::ESC_CONTROL_CHARS
                | op::ESC_C_CONTROL)
                & !op::ESC_LTGT_WORD_BEGIN_END,
            op2: op2::QMARK_GROUP_EFFECT
                | op2::OPTION_RUBY
                | op2::OPTION_ONIGURUMA
                | op2::QMARK_LT_NAMED_GROUP
                | op2::ESC_K_NAMED_BACKREF
                | op2::ESC_G_SUBEXP_CALL
                | op2::ESC_P_BRACE_CHAR_PROPERTY
                | op2::ESC_P_BRACE_CIRCUMFLEX_NOT
                | op2::PLUS_POSSESSIVE_REPEAT
                | op2::PLUS_POSSESSIVE_INTERVAL
                | op2::CCLASS_SET_OP
                | op2::ESC_CAPITAL_C_BAR_CONTROL
                | op2::ESC_CAPITAL_M_BAR_META
                | op2::ESC_V_VTAB
                | op2::ESC_H_XDIGIT
                | op2::ESC_CAPITAL_K_KEEP
                | op2::ESC_CAPITAL_R_GENERAL_NEWLINE
                | op2::ESC_CAPITAL_N_O_SUPER_DOT
                | op2::QMARK_TILDE_ABSENT_GROUP
                | op2::ESC_X_Y_TEXT_SEGMENT
                | op2::QMARK_LPAREN_IF_ELSE
                | op2::ASTERISK_CALLOUT_NAME
                | op2::QMARK_BRACE_CALLOUT_CONTENTS
                | op2::ESC_U_HEX4,
            behavior: GNU_BV
                | behavior::DIFFERENT_LEN_ALT_LOOK_BEHIND
                | behavior::CAPTURE_ONLY_NAMED_GROUP
                | behavior::ALLOW_MULTIPLEX_DEFINITION_NAME
                | behavior::FIXED_INTERVAL_IS_GREEDY_ONLY
                | behavior::ALLOW_INTERVAL_LOW_ABBREV
                | behavior::VARIABLE_LEN_LOOK_BEHIND
                | behavior::WHOLE_OPTIONS
                | behavior::ESC_P_WITH_ONE_CHAR_PROP
                | behavior::ALLOW_CHAR_TYPE_FOLLOWED_BY_MINUS_IN_CC,
            options: Options::NONE,
            meta_escape: b'\\' as u32,
            meta_anychar: 0,
            meta_anytime: 0,
            meta_zero_or_one: 0,
            meta_one_or_more: 0,
            meta_anychar_anytime: 0,
        }
    }

    pub const ONIGURUMA: Self = Self::oniguruma();
}

/// SQL-like variable metas: `%` = `.*`, `_` = `.`
pub fn sql_syntax() -> Syntax {
    let mut s = Syntax::oniguruma();
    s.op |= op::VARIABLE_META_CHARACTERS;
    s.set_meta_char(1, b'_' as u32);
    s.set_meta_char(5, b'%' as u32);
    s
}

/// True when this encoding treats POSIX/Unicode word tests as Unicode.
pub fn unicode_word(enc: Encoding, opt: Options) -> bool {
    enc.is_unicode() && !opt.contains(Options::WORD_IS_ASCII) && !opt.contains(Options::POSIX_IS_ASCII)
}
