# Plan: `symbols` — a shared glyph crate for all House Rust apps

- **Status:** proposed, not started
- **Created:** 2026-08-09
- **Owner:** unassigned
- **Northern star:** [`ratatui/ratatui`](https://github.com/ratatui/ratatui) — see below

---

## Problem

Unicode glyphs (`→ ✓ ✗ ⏱ ↔ ↪ ▲ ≥ ─`) are written as raw literals scattered
across every Rust app we own. Two consequences:

**1. Every file is a corruption site.** On 2026-08-09 `faucet`'s
`crates/faucet-gui/src/pages/distribution.rs` was found with 743 mojibaked
sequences across 9 distinct glyphs — UTF-8 source bytes that had been read as
Windows-1252 and re-saved. Where `→` was intended, users saw its three bytes
rendered as three separate Latin-1 characters (`U+00E2 U+2020 U+2019`). The
repair was mechanical (re-encode Win-1252, decode UTF-8), but nothing prevents
recurrence in any of the other files.

> The corrupted forms are referred to by codepoint throughout this document
> rather than pasted literally — embedding them would make this file itself
> vulnerable to the round-trip it describes.

Current exposure, counted 2026-08-09:

| Repo | Files containing symbol glyphs |
|---|---:|
| mata-master | 955 |
| faucet | 77 |
| mata-maestro | 32 |
| comet | 29 |
| Dial | 3 |

~1,100 files across five repos, each an independent place the same bug can
happen.

**2. No uniformity across apps.** Nothing makes faucet's checkmark the same
glyph as mata's, and nothing pins *presentation* (see below). The same source
renders differently across platforms.

## Northern star

**[`ratatui/ratatui`](https://github.com/ratatui/ratatui)** — specifically its
`src/symbols.rs` module.

It is the closest existing thing to what we want and the bar we should clear.
Capabilities to mirror:

- **Semantic naming over literal glyphs.** Call sites reference a named
  constant describing intent, not the character.
- **Grouped, swappable sets.** Related glyphs are bundled into named variants
  (normal / rounded / double / thick) so a whole visual family is switched in
  one place rather than per-call-site.
- **Flat, dependency-free constants.** No runtime, no allocation, no
  initialization. Just `const` data that inlines.
- **Exhaustive coverage of one domain, and nothing else.** It does box drawing
  and markers thoroughly; it does not drift into text layout or i18n.

Secondary reference: **[`console-rs/console`](https://github.com/console-rs/console)**
for its `Emoji` type, which pairs a glyph with an ASCII fallback and picks
based on what the target can render. That graceful-degradation pattern is the
model for our presentation handling.

What we should deliberately *not* copy from either: both are terminal-first.
Our primary consumers are Dioxus/WebView GUIs rendering HTML, so our
presentation and accessibility concerns differ (see Design).

## Design

### Constants are ASCII escapes, never literal glyphs

```rust
pub const ARROW_RIGHT: &str = "\u{2192}";   // →
pub const CHECK:       &str = "\u{2713}";   // ✓
pub const GTE:         &str = "\u{2265}";   // ≥
```

This is the load-bearing decision. A crate written entirely in ASCII escapes
**cannot** mojibake, because there are no non-ASCII bytes for an encoding
round-trip to corrupt. The glyph exists once, in a form that is immune, instead
of 1,100 times in a form that is not.

### Pin presentation explicitly

Several glyphs we use render as colour emoji or as text depending on platform
and font — `⏱` (U+23F1) most notably, plus `▲` and `✓` in some contexts.
Dioxus desktop means WebView2 on Windows and WKWebView on macOS: identical
source, different glyph.

Constants that are presentation-ambiguous carry an explicit variation selector:

```rust
pub const STOPWATCH: &str = "\u{23F1}\u{FE0E}";  // ⏱ + VS15 → force text
```

This is uniformity that is not achievable by convention across 1,100 files.

### Semantic grouping

Group by role, not by appearance, so intent survives a visual change:

- `status::` — ok, fail, pending, warn
- `nav::` — arrow directions, breadcrumb separators
- `structure::` — box drawing, rules, separators
- `math::` — gte, lte, approx, delta

`status::OK` and `list::BULLET` may both be `✓` today; grouping means changing
one doesn't silently change the other.

### Accessibility helper (feature-gated)

A bare glyph in HTML has no accessible name — a screen reader announces
nothing useful for `✓`. Behind a `html` feature, emit the labelled form:

```rust
symbols::html::labelled(status::OK, "verified")
// -> <span role="img" aria-label="verified">✓</span>
```

Impossible to enforce while glyphs are scattered literals.

## Enforcement

The crate alone is a convention people drift from. Two guards:

1. **Self-test in the crate.** A `#[test]` reads the crate's own source and
   asserts no byte exceeds `0x7F`, plus a test asserting each constant equals
   the scalar value intended. The crate cannot regress into literals.

2. **CI check in every consumer.** A grep that fails the build on any
   non-ASCII byte in `.rs` files. This does double duty: it catches corruption
   *and* it forces new symbols through the crate, because a raw glyph will not
   pass review.

Without (2) this decays within a year.

## Distribution

These are separate repos, not one workspace, so a path dependency will not
reach across them.

**Decision: git dependency.**

```toml
symbols = { git = "ssh://git@github.com/<org>/symbols", tag = "v0.1.0" }
```

Simplest mechanism that works today, and version-pinned per app so upgrades
are deliberate. A private cargo registry is ergonomically nicer but is real
setup; vendored copies defeat the purpose entirely.

## Scope boundaries

This is **not** a text library. Explicitly out of scope:

- i18n / localization
- string formatting, pluralization
- font loading or bundling
- anything with a runtime or a non-trivial dependency

Target size: ~100 lines of constants, a semantic grouping, two tests.

## Migration

Honest assessment: **this is prevention, not a cure.**

The ~1,100 existing files keep their literals and stay corruptible until
migrated. A wholesale codemod across five repos is a larger job than it
sounds and carries its own corruption risk. Recommended path:

1. Land the crate.
2. Adopt for all *new* code.
3. Fix files as they are touched for other reasons.
4. Only consider a bulk codemod per-repo, never all at once, and only with the
   CI check already in place to verify the result.

## Open questions

- **CSS cannot consume a Rust crate.** Glyphs in `styles.css`
  (`content: "→"`) stay exposed unless a build step generates CSS custom
  properties from the same source. In scope, or accepted as-is?
- **Repo host and org path** for the git dependency — needs deciding before
  the first consumer wires up.
- **Does mata-master's 955-file footprint** contain glyph families beyond
  faucet's 9? A survey should precede the constant list so v0.1 isn't
  immediately insufficient.

## First step

Scaffold the crate with the constants, the ASCII self-test, and the CI grep;
wire `faucet` as the first consumer so it can be judged on one repo before it
touches mata-master.
