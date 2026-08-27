# Plan: `thoth::expressions` (rusty_expressions) -- Oniguruma remade in Rust

- **Status:** Phase 0--4 complete. Match-equivalent to libonig on every
  harvested vector and every differential gate; **~3x faster than libonig on
  search** (ours/onig 0.32-0.33, 22-23 of 23 benchmark cases ours-faster, 0
  onig-faster) and ~2x on compile. Ready to consider the v0.4.0 tag.
- **Repo:** https://github.com/Remade-With-Rust/thoth
- **Created:** 2026-08-26
- **Updated:** 2026-08-27
- **Owner:** unassigned
- **Product nickname:** rusty_expressions (module path stays `thoth::expressions`)
- **Northern star:** [`kkos/oniguruma`](https://github.com/kkos/oniguruma) 6.9.10 / `doc/RE` 6.9.11 (archived 2025-04-24)
- **README format:** [`rusty_dds`](https://github.com/Remade-With-Rust/rusty_dds)

---

## 1. Problem

House apps, and every mesh node they will run on, need a regular-expression engine
that is **Oniguruma-class** -- named groups, look-around, backreferences, subexp
calls, per-regex encodings, pluggable syntax -- and that still passes the house
one-line test:

> Could this ship as-is to a user who assumes their data is theirs alone, onto a
> machine you do not own, with no C toolchain anywhere in the build?

Three facts make the current options fail that sentence.

**1. Oniguruma itself is finished.** The C project ended 2025-04-24. It was the
engine behind MRI Ruby 1.9+, PHP `mb_ereg`, and `jq`. The last release (6.9.10)
carries Unicode 16.0 and years of OSS-Fuzz / Coverity fixes. There will be no
further security patches from upstream. A C copy in our tree is a CVE surface we
cannot close, and a `*-sys` crate is a C toolchain on every mesh node.

**2. The crates.io `onig` crate is FFI, not a remake.** It links `libonig`. That
is a binding, not a replacement. It cannot target `wasm32-unknown-unknown`, it
cannot ship without a C compiler, and it inherits every remaining C bug.

**3. rust-lang/`regex` is a different contract.** It is a linear-time automata
engine that *deliberately* omits backreferences, look-around, recursion, and
possessive / atomic groups -- the features Oniguruma exists to provide.
`fancy-regex` covers a subset on top of `regex`, but it is not Oniguruma: no
per-regex encodings, no `OnigSyntaxType` dialects (Ruby / Perl / Python / Java /
POSIX / GNU / Emacs / grep / ASIS), no subexp-call (`\g<>`), no absent
expressions, no callouts, no RegSet, no Unicode-16 property tables matching
`doc/RE`.

Consequence: any house surface that needs Ruby/Perl-class patterns today either
pulls C, or silently changes match semantics by swapping in `regex`. Both are
defects.

This is the regex counterpart to remade_ffmpeg_rs: **rebuild the finished C
library in pure Rust, gate against its own test corpus, keep the documented
behavior.**

## 2. Goals / non-goals

### Goals

- Pure-Rust Oniguruma remake: parse -> compile -> bytecode VM -> match/search.
- Feature `expressions`; module `thoth::expressions` (product nickname
  rusty_expressions). Requires `alloc`.
- `no_std` + `alloc` core; `wasm32-unknown-unknown` checked. No C, no FFI, no
  `onig-sys`.
- Encoding is a first-class value on each compiled regex (Oniguruma's headline).
  Phase 0 ships ASCII + UTF-8; the `Encoding` trait is the seam for the rest.
- Syntax is a first-class value (`Syntax`), not a process global. Phase 0 ships
  `Syntax::ONIGURUMA` (default). Flag tables from `doc/SYNTAX.md` stay the
  configuration surface.
- Public API is Rust-native (`Regex`, `Region`, `MatchParam`, `Syntax`,
  `Encoding`, `Options`). Compile is separate from search. Compiled `Regex` is
  `Send + Sync`.
- Match-equivalence oracle: harvested Oniguruma test vectors. Same match
  start/end and capture byte-offsets as 6.9.10/6.9.11 on that corpus.
- `MatchParam` retry / stack limits from Phase 0 so untrusted patterns cannot
  hang a wasm or mesh node (Oniguruma's `onig_set_retry_limit_in_match`).
- ASCII crate source (existing thoth gate). Unicode property tables are
  generated ASCII Rust, never pasted glyphs.
- rusty_dds-style README capability rows; this plan is the backlog.

### Non-goals

- Linking or vendoring libonig / Onigmo / PCRE2 / `onig` / `onig_sys`.
- A C ABI (`onig_new` / `regex_t`) in v0.1. Optional `compat` feature is a
  later split, not a Phase 0 deliverable.
- rust-lang/`regex` as an implementation dependency. Consult it for algorithms;
  do not wrap it. Oniguruma semantics win every disagreement.
- POSIX / GNU C API shims (`onigposix.h`, `oniggnu.h`) in v0.1.
- Callouts of contents / name (`(?{...})`, `(*name)`) in Phase 0 -- they are C
  callbacks in the original; Phase 3 gets a Rust closure seam.
- East-Asian encodings (EUC-JP, Shift_JIS, Big5, GB18030, ...) in Phase 0.
- Bulk-replacing `regex` in existing house apps. Different contract; migrate
  only call sites that need Oniguruma-class syntax.
- Separate published crate named `rusty_expressions` (nickname only until a
  later split, same as rusty_tokens / rusty_a11y).
- Claiming linear-time matching. Oniguruma is a backtracking NFA; so are we.
  Limits are the mitigation, not a Thompson rewrite.

## 3. Architecture

```text
thoth::expressions          [feature = "expressions"]     -- rusty_expressions
  encoding   -- Encoding trait + ASCII + UTF-8 (Phase 0)
  syntax     -- Syntax { op, op2, behavior, options, meta }
  parse      -- pattern bytes -> AST (encoding-aware)
  compile    -- AST -> packed bytecode + capture map
  exec       -- backtracking VM (match / search)
  region     -- capture slots, named groups
  param      -- MatchParam (stack limit, retry limits)
  error      -- compile / match errors (no C err-buf)

later (same module, later phases)
  scan       -- find-all (onig_scan)
  set        -- multi-pattern RegSet
  callout    -- Rust closure seam
  encoding::* -- UTF-16/32, ISO-8859-*, EUC-*, SJIS, Big5, GB18030, ...
  syntax::*  -- Perl, Python, Java, POSIX, GNU, Emacs, Grep, ASIS
```

Pipeline (Oniguruma's three files, as Rust modules):

```text
pattern bytes + Encoding + Syntax + Options
        |  parse
        v
       AST
        |  compile (+ peephole)
        v
    bytecode  ----exec---->  Region (offsets in haystack bytes)
```

Northern-star capabilities to mirror from Oniguruma (without copying C source
or a C ABI):

- Per-regex `Encoding` and `Syntax`
- Bytecode VM + backtrack stack (not a Thompson NFA)
- Named groups, numbered groups, capture history
- Look-ahead / look-behind, atomic groups, possessive quantifiers
- Subexp-call (`\g<>` / `\g''`) and absent expressions
- Unicode properties + case-fold data matching Unicode 16.0
- Search vs match vs scan vs RegSet
- Retry / stack limits on the match param

Secondary references (algorithms only, never the oracle):

- rust-lang/`regex` + `regex-automata` -- linear-time subset; different contract
- `fancy-regex` -- backref / look-around on top of `regex`; not Oniguruma
- MRI Onigmo -- a fork; do not mix oracles

Sibling plans: [symbols-crate.md](symbols-crate.md),
[tokens-crate.md](tokens-crate.md), [a11y-crate.md](a11y-crate.md).

### Layout in this repo

```text
src/expressions/
  mod.rs
  encoding/{mod.rs, ascii.rs, utf8.rs}
  syntax.rs
  ast.rs
  parse.rs
  compile.rs
  opcode.rs
  exec.rs
  region.rs
  param.rs
  error.rs
tests/expressions.rs          -- unit + harvested-vector gate
tests/data/oniguruma/         -- harvested fixtures (no C)
tools/onig-oracle/            -- OPTIONAL bring-up only; never a dep
```

`tools/onig-oracle` may compile C Oniguruma **once**, offline, to regenerate
fixtures. It is not a workspace member of the shipping crate and must not
appear in `Cargo.toml` dependencies.

`tools/onig-bench` is the side-by-side correctness + timing harness:

```sh
cargo run --release --manifest-path tools/onig-bench/Cargo.toml
cargo run --release --manifest-path tools/onig-bench/Cargo.toml --features oracle
```

Fixture-only mode compares `thoth::expressions` to harvested JSON (exit 1 on
region mismatch). `--features oracle` also links crates.io `onig` (libonig)
for live C search/compile timings. That C link is bench-only.

## 4. Capability backlog (phased)

### Phase 0 -- Scaffold (this crate)

The smallest engine that is recognizably Oniguruma: UTF-8, default syntax,
compile + match/search, numbered captures, retry limits.

- [x] Feature `expressions` (implies `alloc`); `src/expressions/` module tree
- [x] `Encoding` value type + `Ascii` + `Utf8` (method seam, not a stringly enum)
- [x] `Syntax` struct + `Syntax::ONIGURUMA` flags from `doc/SYNTAX.md`
- [x] `Options` bitflags used at compile and at search
- [x] Parser: literals, `.`, `*+?` / `{n,m}`, `|`, `(...)` / `(?:...)`,
      `[...]` (ranges, negate), anchors `^ $ \A \z \b`, escapes `\n \t \xHH`
      `\x{...} \w \d \s` and their negations
- [x] Compiler: AST -> bytecode (literal, any, class, split/jump, save, match)
- [x] Exec: `Regex::is_match`, `search`, `find` at a position; `Region` with
      numbered captures
- [x] `MatchParam` with stack-depth and retry-in-match / retry-in-search limits
      (defaults on; 0 = unlimited, same as Oniguruma)
- [x] Error type with position in the pattern; no `unwrap` on user input
- [x] ASCII self-test covers the new module
- [x] Harvest Oniguruma `test/` vectors that Phase 0 can answer; store under
      `tests/data/oniguruma/` as ASCII JSON/UTF-8 fixtures
- [x] `cargo test --features expressions` green; `cargo check --target
      wasm32-unknown-unknown --no-default-features --features expressions`
- [x] Plan doc + README rows (capability table + architecture)
- [x] Do **not** tag until Phase 0 exit is green. Next tag is **v0.4.0**
      (expressions scaffold) when ready to pin consumers

**Exit:** compile + search of the Phase 0 syntax subset is match-equivalent to
harvested Oniguruma vectors on ASCII+UTF-8; crate source stays ASCII; wasm
check is green; no C in the shipping graph.

### Phase 1 -- First consumer

- [x] Path dep in one house UI or CLI that already wants Oniguruma-class
      patterns (search box, validator, or a `thoth` example)
- [x] Call sites go through `thoth::expressions`, not `regex` / `onig`
- [x] `cargo check` of that consumer green with `default-features = false` if
      it is a library

**Exit:** one real consumer compiles and matches the same strings the C engine
matched for its fixtures. Example-only is acceptable if no app is ready; the
example must be an op (callable from a test), not only a UI handler.

### Phase 2 -- Oniguruma-complete on UTF-8

Default syntax, Unicode encoding, the features `doc/RE` lists as original.

- [x] Look-ahead / look-behind (fixed and variable width)
- [x] Possessive quantifiers, atomic groups `(?>...)`
- [x] Named groups `(?<name>...)` / `(?'name'...)`, named backrefs `\k<name>`
- [x] Numbered / relative backrefs; capture-group option interaction
      (`DONT_CAPTURE_GROUP` / `CAPTURE_GROUP`)
- [x] Subexp-call `\g<n>` / `\g<name>` with the left-recursion rule
      (static left-recursion reject deferred: `testc.c:left-recursive-g-static-error`)
- [x] Absent repeater / expression / stopper (`(?~...)`)
- [x] Conditional `(?(cond)then|else)` and backref-validity checker
- [x] Unicode properties `\p{...}` / `\pX` / POSIX brackets; Unicode 16.0
      committed UCD tables (`src/expressions/ucd16.rs`; generator
      `tools/gen_ucd16.py`)
- [x] Case fold (Unicode + `IGNORECASE_IS_ASCII`)
- [x] Text-segment `\X` `\y` `\Y` (extended grapheme default;
      `Options::TEXT_SEGMENT_WORD` for word segments)
- [x] Isolated / whole options `(?imxWDSPy-...)`, `(?CIL)`
- [x] `FIND_LONGEST`, `FIND_NOT_EMPTY`, `MATCH_WHOLE_STRING`, NOTBOL / NOTEOL
- [x] `\K`, `\G`, `\R`, `\O`, `\N`, `\h` `\H`
- [x] Remaining harvested UTF-8 Oniguruma tests green
      (`tests/data/oniguruma/deferred.txt` is a closed log)
- [x] Capture-history tree (`(?@...)` / `Region::history` / `traverse_history`)
- [x] Compile-time left-recursive `\g` reject (`ErrorKind::NeverEndingRecursion`)

**Exit:** UTF-8 + `Syntax::ONIGURUMA` is match-equivalent on the harvested
corpus. Named gaps are closed (empty `deferred.txt`) and gated by tests.

### Phase 3 -- Dialects, encodings, scan/set, split

- [x] Built-in syntaxes: Perl, Perl_NG, Python, Java, POSIX basic/extended,
      GNU regex, Emacs, grep, ASIS (flag tables from `doc/SYNTAX.md`)
- [x] UTF-16BE/LE, UTF-32BE/LE
- [x] ISO-8859-1..16, KOI8-R, CP1251
- [x] EUC-JP / EUC-TW / EUC-KR / EUC-CN, Shift_JIS, Big5, GB18030
      (Unicode round-trip maps from WHATWG indexes; `tools/gen_cjk_maps.py`)
- [x] `scan` (find-all) and `RegSet` (multi-pattern, same encoding)
- [x] Callout seam: `CalloutFn` (`fn` pointer, `Send + Sync`) in progress /
      retraction; built-in `(*SKIP)` / `(*COUNT)` as named callouts;
      `MatchParam.count` persists across searches
- [x] User-defined Unicode property hook
- [x] Variable meta characters (SQL `%` / `_`)
- [x] Optional `compat` feature: pure-Rust `extern "C"` `onig_new` /
      `onig_search` / `OnigRegion` (no libonig, no C toolchain)
- [x] Expand only from real call sites after that
- [x] Split: **remain `src/expressions/` in thoth** until after v0.4.0
      (same in-tree doctrine as tokens/a11y). Do not create
      `github.com/Remade-With-Rust/rusty_expressions` unless explicitly asked.
- [x] crates.io: name `rusty_expressions` is **available** (registry API 404).
      Do not publish until an explicit split; next tag is still v0.4.0.

**Exit:** encoding x syntax matrix that Oniguruma documented is present or
explicitly deferred with a reason; scan/set work; callouts have a Rust seam.

### Phase 3.5 -- Search prefilter (done 2026-08-27)

Measured against live libonig (`tools/onig-bench --features oracle`), 100 KB
haystack, no match. `search()` was entering the VM at **every** start position
for any class-led pattern: `leading_ascii_byte` only understood `Inst::Char` /
`Inst::Literal`, so `[0-9]`, `\d`, `\p{Lu}`, `\w` got no prefilter at all.
Deterministic counters (`--features count`) showed the shape exactly -- for
`[0-9]` over 100 KB: `search_pos` 100_001, `engine_new` 100_001, `byte_scan` 0.
The literal-led control `q[0-9]` on the same haystack: 1, 0, 100_000.

- [x] `Program::lead: Option<Lead>` -- 256-bit first-byte set, exact over ASCII
      (probes the real matcher via `class_hit_in`, so a prefilter can never
      disagree with the VM), conservative above U+007F
- [x] `Lead::single` keeps the memchr-shaped equality loop for one-byte sets
- [x] Recomputed by `define_user_property` (a user property can change what an
      ASCII codepoint matches); skipped under `IGNORECASE` and `min_len() != 1`
- [x] `class_hit` / `item_hit` extracted to free `class_hit_in` / `item_hit_in`
      so the prefilter and the VM share one definition of membership
- [x] Dropped the per-character `class.clone()` in the `Inst::Class` arm by
      reborrowing `prog` at `'a` in `run_stop` (it was a heap allocation per
      character tested)
- [x] `consume_class`: one decode instead of two (`decode_len` reusing the
      length from `mbc_len`, on the `hay_end`-bounded slice -- `mbc_to_code`
      was re-deriving the length from an unbounded slice), plus an ASCII
      fast path for `ascii_transparent()` encodings
- [x] Gate: `tools/onig-bench --example prefilter_diff` -- 12_800 generated
      pattern x haystack pairs vs libonig, cross-checked against `find_at`
      (which bypasses the prefilter) to separate a prefilter false negative
      from a pre-existing semantic diff. **0 prefilter bugs.**

Result, 100 KB no-match scan (ours / libonig):

| pattern | before | after | vs libonig before | vs libonig after |
|---|---|---|---|---|
| `[0-9]+` | 23.2 ms | 47 us | 64x slower | **7.7x faster** |
| `\d` | 23.0 ms | 71 us | 10.5x slower | **25x faster** |
| `\p{Lu}` | 30.9 ms | 50 us | 14x slower | **28x faster** |

Match-heavy path (30 KB, ~40% digits, find-all): `[0-9]+` 8.80 ms -> 3.25 ms,
`\p{Nd}+` 11.10 ms -> 3.59 ms; now 1.05-1.66x of libonig. Harvested-corpus
total moved from `ours/onig=1.081` to `0.870`. Compile cost rose ~40% (the
128-codepoint ASCII probe) and is still ~1.6x faster than libonig.

### Phase 4 -- Correctness defects (found and closed 2026-08-27)

All found by differential testing against live libonig, all closed and gated.
`onig-bench --features oracle` is **50/50, diffs=0, oracle_skew=0** (was 41/42
with one diff and one fixture that recorded our own bug as expected).

- [x] **Greedy repeat aborted the process at ~2 KB.** `repeat_rec` recursed
      once per repetition, so backtrack depth was native stack depth; `\w+`,
      `[a-z]+`, `.+`, `\p{L}+` over 2000 bytes killed the process. The
      `stack_limit` guard could never fire because `try_body` called
      `pop_stack()` *before* recursing, so the counter returned to baseline
      every iteration. Rewritten iteratively over a heap trail, same search in
      the same order; `check_repeat_depth` now bounds the trail, so
      `stack_limit` is real (fires at exactly `limit`, `0` = unlimited).
      `.+` over **1 MB** now matches in ~16 ms.
- [x] **`\xHH` / `\x{...}` were dropped inside a character class.**
      `[\x{00e9}]` was a class of `0`, `e`, `9`. New `class_escape_char` routes
      class escapes through the same readers `parse_escape` uses, and `\p{...}`
      is now a class item. Also fixed `\uHHHH`, `\o{...}`, `\cX`, `\r \f \v \a
      \e`, `\b`-as-backspace, and escaped range endpoints.
- [x] **`(*FAIL)` / `(*MISMATCH)` were unimplemented and silently succeeded.**
      Now fail where they stand.
- [x] **`(*SKIP)` fired on progress instead of retraction.** It now succeeds
      going forward and only records where it was reached; if the whole attempt
      at that start fails, the search resumes **at** that point (it was
      resuming one character past it). `(*ERROR)` and `(*TOTAL_COUNT)` added.
- [x] **`\g<n>` did not propagate the called group's capture.** `subexp_call`
      ran the group body *between* its `Save` pair, so a call never wrote the
      slot. It now runs the Save pair inclusively -- and for a **recursive**
      call (call site inside the callee's own span) restores the slots on the
      way out, because Oniguruma reports the outermost invocation:
      `(a)\g<1>` on "aa" gives g1=1..2, `(?<a>x\g<a>?y)` on "xxyy" gives
      g1=0..4.
- [x] **Named groups did not disable numbered ones.** `has_named` was set as
      parsing progressed, so `(x)` in `(x)(?<a>y)` still captured. A whole-
      pattern pre-scan (`scan_has_named_group`) now decides it up front, as
      Oniguruma does.
- [x] **`assert_harvested` compared only the whole-match range**, dropping
      `captures` -- which is exactly how the `\g<>` bug stayed green. It now
      compares every group.
- [x] **`phase3.json:callout-skip-fail` recorded our bug as expected.**
      Re-harvested from libonig and joined by eight new vectors covering each
      defect above. Both fixture loaders now decode `\uXXXX` (an escaped
      codepoint silently decayed to the literal text `uXXXX`).

Gates now standing:

| Gate | What it catches |
|---|---|
| `onig-bench --features oracle` | any fixture drift vs libonig, and any fixture that disagrees with libonig |
| `--example prefilter_diff` | prefilter false negatives, 12_800 generated pairs, cross-checked against `find_at` |
| `--example verbs` / `gcall` / `classesc` | callout verbs, subexp-call captures, class escapes |
| `--example stress_repeat` / `one` | the repetition abort, at sizes libonig handles |
| `--example limits` | `stack_limit` fires at the boundary |

Known and deliberate: a lazy quantifier that cannot match (`[a-z]+?x` over a
long haystack) is O(n^2) in any backtracker; it returns a graceful
`RetryLimitSearch` rather than running forever. Oniguruma is a backtracking
NFA and so are we -- limits are the mitigation, as section 2 says.

### Phase 5 -- Search optimization (done 2026-08-27)

Measured against live libonig, 64 KB corpora, ABBA-interleaved, medians, with
a null arm establishing the noise floor (0.0% arm skew, 1-4% spread). Modeled
work fell from 159_668_485 to ~24M (-85%) and heap allocations from 432_844 to
~53k across the benchmark workload.

Search-position filters, each provably unable to change a match:

- `Program::lead` -- 256-bit first-byte set from a walk of the whole program
  (through `Save`, anchors, option pushes and look-around; unioned across
  `Split`), not just the leading instruction
- `Program::req_lit` (`optimize.rs`) -- a byte sequence every match must
  contain, with its distance range, and **run-anchored** when preceded by an
  unbounded class run that cannot match its first byte. That is what makes an
  unbounded distance usable: the match must begin at or after the start of the
  run ending at the literal. `[\w.]+@[\w.]+\.\w+` went 48_201 start positions
  to 242
- `Program::anchored_bol` -- `^`-anchored patterns only consider line starts,
  found by scanning for a newline; fused with the required-literal check when
  the literal sits at the match start

Per-attempt and per-character work:

- One `Engine` per search, not per start position (342_852 -> 26_306)
- Inline capture storage (16 slots) -- `Engine::new` allocates nothing
- Compile-time tables: repeat shapes, class ASCII bitmaps (built under the
  options live at that pc), literal bytes, group Save-spans, literal fast path
- Follow-literal-guided backtracking: a greedy class run only tries lengths
  that leave the required next byte in place; when the class cannot match that
  byte at all, only the maximal length is viable
- Tight ASCII byte loop for class runs; `char_before`, `next_pos` and the
  encoding predicates all take byte fast paths
- `rusty_alloc` as the global allocator is worth a further ~21% on our arm
  (0.36 -> 0.29 in an A/B); the bench enables it by default

**Correctness gates** (all zero):

| Gate | Coverage |
|---|---|
| `onig-bench --features oracle` | 50/50 harvested vectors, flags fixture-vs-libonig skew |
| `--example prefilter_diff` | 25_600 pairs; cross-checks every skip against `find_at` |
| `--example constructs_diff` | 21_500 pairs over bounded-subprogram constructs |
| `--example audit` | 30_092 checks over encodings, options, syntaxes, user properties, capture spill and non-zero search starts |
| `verbs` / `gcall` / `classesc` / `emptyline` | callout verbs, subexp captures, class escapes, line anchors |

Bugs these gates caught while optimizing, each fixed:

- `(?m)^$` matched at end-of-string; a trailing newline begins no line
- `bol_guaranteed` was unsound as first written -- the first-byte filter moved
  `pos` off the line start the anchor scan had established (151 cases)
- **User-defined properties invalidated more than the first-byte filter.**
  `analyze` builds the class bitmaps, required literal and repeat shapes;
  `define_user_property` refreshed only `lead`, so the stale bitmap let the
  required-literal filter skip past real matches. All class-derived tables are
  now rebuilt whenever the property set changes

## 5. Design decisions (locked for v0.4 / Phase 0)

### Feature-gated module, not a new workspace crate (yet)

```toml
thoth = { git = "https://github.com/Remade-With-Rust/thoth.git", tag = "v0.4.0", features = ["expressions"] }
# libraries already choosing an allocator:
# thoth = { git = "...", tag = "v0.4.0", default-features = false, features = ["expressions"] }
```

`expressions` does not turn on `rusty-alloc`; the crate default still does.
Libraries keep `default-features = false`.

### Rust-native API, Oniguruma semantics

```rust
use thoth::expressions::{Encoding, Options, Regex, Syntax};

let re = Regex::new(
    "ca+t",
    Options::NONE,
    Encoding::UTF8,
    Syntax::ONIGURUMA,
)?;
assert!(re.is_match("caaat"));
let m = re.search(b"one cat two")?;
assert_eq!(m.range(), 4..7);
```

Offsets are **byte offsets in the haystack**, same as Oniguruma `OnigRegion`.
Haystack type is `&[u8]`. UTF-8 convenience wrappers (`&str`) are allowed on
`Encoding::UTF8` only.

### Encoding is a trait, not a stringly enum we cannot extend

Phase 0 implements ASCII and UTF-8. Further encodings implement the same trait
(mbc length, codepoint in/out, case-fold, ctype, reverse-match legality). User
encodings are allowed later the way Oniguruma allowed `OnigEncodingType`.

### Syntax is data

`Syntax` is the `OnigSyntaxType` trio (`op`, `op2`, `behavior`) plus default
`Options` plus optional meta-char table. Built-ins are associated constants.
A caller who needs SQL wildcards builds a `Syntax` value; we do not special-case
them in the parser.

### Oracle is harvested vectors, not live libonig

A fixture is:

```text
syntax, encoding, options, pattern, haystack, start, range,
expected: mismatch | { start, end, captures: [[beg, end] | unset] }
```

Phase 0 fails the build if any harvested vector in the Phase 0 allow-list
diverges. Adding a feature in Phase 2 means moving its vectors from "deferred"
to "gated", never rewriting the expected region to match us.

### We remake the spec, we do not copy the C

Oracle: `doc/RE`, `doc/API`, `doc/SYNTAX.md`, harvested `test/` output.
Oniguruma license is BSD; thoth stays MIT. Do not import `regparse.c` /
`regcomp.c` / `regexec.c`. Opcode names may follow `regint.h` where that makes
the bring-up trace comparable.

### Limits are on by default

Untrusted pattern + hostile haystack is a hang on a backtracker. Phase 0
`MatchParam` carries:

| Knob | Oniguruma API | Default |
|---|---|---|
| match-stack depth | `onig_set_match_stack_limit_size` | finite, documented |
| retry-in-match | `onig_set_retry_limit_in_match` | finite, documented |
| retry-in-search | `onig_set_retry_limit_in_search` | finite, documented |

Hitting a limit is an error, not a silent mismatch.

### Unicode tables are generated

Unicode 16.0, matching Oniguruma 6.9.10. A Rust generator reads UCD and emits
ASCII `.rs` data files. Regeneration is explicit; committed output is the
source of truth so `wasm32` / mesh builds never need the UCD at compile time.

## 6. Open questions

| Question | Phase 0 decision |
|---|---|
| In-tree module vs new repo | **`src/expressions/`** in thoth until after v0.4.0 |
| C ABI / `onig_*` names | **`compat` feature** (pure Rust `extern "C"`; no libonig) |
| rust-lang/`regex` as a dep | **No** |
| Encodings in v0.4 | ASCII + UTF-8 in Phase 0; Phase 3 encodings with Unicode maps |
| Syntaxes in v0.4 | Phase 0 `Syntax::ONIGURUMA`; Phase 3 built-ins |
| Unicode version | **16.0** (Oniguruma 6.9.10); committed `ucd16.rs` |
| Callouts | Phase 3; `(*COUNT)` on `MatchParam.count` |
| Live libonig in CI | **No**; harvested fixtures only |
| Next tag | **v0.4.0** after Phase 0 exit; do not tag from this work |
| crates.io name `rusty_expressions` | **available** (API 404); do not publish until an explicit split |

## 7. Migration doctrine

**This is a new capability, not a drop-in for `regex`.**

1. Land the engine (Phase 0) behind `features = ["expressions"]`.
2. Adopt for *new* code that needs Oniguruma-class syntax or per-regex
   encodings.
3. Leave rust-lang/`regex` in place where linear-time and the restricted
   syntax are the point (untrusted hot paths that must not backtrack).
4. Replace a call site only when its pattern uses a feature `regex` refuses,
   or when the match must agree with Ruby/PHP/jq/Oniguruma.
5. Bulk swap is forbidden: it would change complexity class and match
   semantics at once.

Prevention of C: new house regex work does not add `onig` / `onig_sys` /
`pcre2-sys`. If a crate already links them, migrate on touch.

## 8. First step

Scaffold `src/expressions/` with `Encoding::{Ascii,Utf8}`, `Syntax::ONIGURUMA`,
a parser/compiler/VM that handles the Phase 0 subset, `MatchParam` limits, the
harvested-vector test file, ASCII + wasm checks, and README rows.

Do not tag. Phase 1 starts when Phase 0 exit is green.
