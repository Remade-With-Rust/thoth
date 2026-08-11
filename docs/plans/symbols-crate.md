# Plan: `thoth` — shared glyph / symbols crate for House Rust apps

- **Status:** Phase 2 substantially complete; mata-master on-touch migration ongoing; Phase 3 next
- **Created:** 2026-08-09
- **Updated:** 2026-08-10
- **Owner:** unassigned
- **Northern star:** [`ratatui/ratatui`](https://github.com/ratatui/ratatui) `symbols` module
- **README format:** [`rusty_dds`](https://github.com/Remade-With-Rust/rusty_dds)

---

## 1. Problem

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

## 2. Goals / non-goals

### Goals

- Semantic named constants over literal glyphs (`status::OK`, not a raw check).
- Constants expressed as ASCII `\u{…}` escapes only — the crate source cannot mojibake.
- Explicit VS15 (`U+FE0E`) on presentation-ambiguous glyphs for WebView uniformity.
- Group by role: `status`, `nav`, `structure`, `math`, `list`.
- Optional `html` feature for accessible labelled spans.
- Self-test (ASCII gate + scalar values) and a consumer CI grep script.
- rusty_dds-style README and phased plan tracking.

### Non-goals

- i18n / localization
- string formatting, pluralization
- font loading or bundling
- runtime, allocation, or non-trivial dependencies
- Bulk-migrating all ~1,100 existing files in v0.1 (prevention first)

## 3. Architecture

```text
┌─────────────────────────────────────────────────────────┐
│  thoth                                                  │
│                                                         │
│  symbols::status     — ok / fail / warn / timer / … ✅  │
│  symbols::nav        — arrows / hooks / collapse     ✅  │
│  symbols::structure  — rules / tree lines            ✅  │
│  symbols::math       — gte / lte / approx / times    ✅  │
│  symbols::list       — bullet / middot               ✅  │
│  symbols::html       — labelled <span>   [feature]   ✅  │
└─────────────────────────────────────────────────────────┘
```

Northern-star capabilities to mirror from ratatui (without copying terminal-first
or literal-glyph source style):

- Semantic naming over literal glyphs
- Grouped related sets
- Flat, dependency-free `const` data
- Exhaustive coverage of one domain only

Secondary reference: [`console-rs/console`](https://github.com/console-rs/console)
`Emoji` graceful-degradation pattern → our VS15 presentation pinning.

## 4. Capability backlog (phased)

### Phase 0 — Scaffold (this crate)

- [x] Package `thoth` with rusty_dds-format README
- [x] `symbols::{status,nav,structure,math,list}` constants as `\u{…}` only
- [x] VS15 on presentation-ambiguous glyphs (stopwatch, alarm, warn, check, triangles)
- [x] Feature-gated `symbols::html::labelled`
- [x] ASCII self-test + scalar-value tests
- [x] Consumer CI helper: `scripts/check-ascii-rs.sh` (+ PowerShell twin)
- [x] Faucet glyph survey informs v0.1 constant list (mojibake-9 + high-frequency extras)
- [x] `cargo test` / `cargo test --features html` green on clean checkout
- [x] Tag `v0.1.0` when ready to pin consumers

**Exit:** `cargo test` passes; crate source has zero bytes `> 0x7F`; README install
path documented. **Met 2026-08-10.**

### Phase 1 — First consumer (faucet)

- [x] Path dep in faucet workspace (`thoth = { path = "../thoth" }`)
- [x] Migrate faucet-gui UI call sites to `thoth::symbols::*` (distribution, settings, enrich, acquisition, run_events)
- [x] ASCII-scrub faucet-gui `src/` (including former comment glyphs) so the consumer gate can enforce
- [x] Consumer ASCII CI scripts under `faucet/scripts/check-ascii-rs.{sh,ps1}` (scoped to `crates/faucet-gui/src` for now)
- [x] `cargo check -p faucet-gui` green

**Exit:** faucet builds; mojibake-9 + high-frequency UI glyphs in faucet-gui use thoth;
CI grep is wired for the GUI crate. **Met 2026-08-10.**

**Platform note:** thoth itself is verified `no_std` + `wasm32-unknown-unknown` (with and without `html`). Faucet desktop remains the first consumer; web/wasm apps can depend on the same crate without change.

### Phase 2 — Rollout to remaining apps

- [x] Dial — `thoth` workspace dep; dial-poison migrated; ASCII gate on `crates/`
- [x] comet — host/cli/pairtest migrated; comment scrub; ASCII gate on `crates/`
- [x] mata-maestro — web UI + api tests migrated; ASCII gate on `src/` + `api/`
- [x] mata-master — workspace `thoth` dep + scripts + Coding Requirements bullet (adopt for new code / migrate on touch; 176 code-glyph files deferred from bulk rewrite)
- [ ] Optional: bulk-migrate mata-master UI packages package-by-package with CI grep already green per package

**Exit:** Dial, comet, and mata-maestro depend on thoth with UI/log glyphs migrated;
mata-master can consume thoth for all new glyphs. **Met 2026-08-10** (mata-master bulk
migration remains on-touch).

### Phase 3 — Hardening (optional)

- [ ] Decide CSS story: generate custom properties from the same constants, or accept CSS as out of scope
- [ ] Survey mata-master for glyph families beyond faucet's set; expand constants without breaking semver where possible
- [ ] Publish path: keep git tags, or move to Remade-With-Rust org / crates.io later

## 5. Design decisions (locked for v0.1)

### Constants are ASCII escapes, never literal glyphs

```rust
pub const ARROW_RIGHT: &str = "\u{2192}";   // →
pub const CHECK:       &str = "\u{2713}\u{FE0E}";  // ✓ + VS15
```

### Pin presentation explicitly

```rust
pub const STOPWATCH: &str = "\u{23F1}\u{FE0E}";  // force text presentation
```

### Semantic grouping

| Module | Role |
|---|---|
| `status` | ok, fail, warn, pending/timer, play, stop, live |
| `nav` | arrow directions, hooks, collapse |
| `structure` | box drawing, rules, tree |
| `math` | gte, lte, approx, times |
| `list` | bullet, middot |

### Accessibility helper (feature `html`)

```rust
thoth::symbols::html::labelled(status::OK, "verified")
// -> <span role="img" aria-label="verified">…</span>
```

### Distribution

```toml
thoth = { git = "https://github.com/Ttimmahlax/thoth.git", tag = "v0.1.0" }
# optional a11y helper:
# thoth = { git = "https://github.com/Ttimmahlax/thoth.git", tag = "v0.1.0", features = ["html"] }
```

## 6. Open questions

| Question | v0.1 decision |
|---|---|
| CSS `content:` glyphs | **Accepted out of scope** for Phase 0–1; revisit in Phase 3 |
| Repo host / package name | **This repo** (`Ttimmahlax/thoth`), package `thoth`, API under `thoth::symbols` |
| mata-master glyph survey | Deferred to Phase 3; faucet survey is enough for v0.1 |

## 7. Migration doctrine

**This is prevention, not a cure.** Existing literals stay until touched.

1. Land the crate (Phase 0).
2. Adopt for all *new* code.
3. Fix files as they are touched for other reasons.
4. Bulk codemod per-repo only with CI grep already in place.

## 8. First step (done when Phase 0 exit is green)

Scaffold the crate with the constants, the ASCII self-test, the CI grep, and a
rusty_dds-format README; wire `faucet` next (Phase 1).
