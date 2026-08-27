# thoth

[![Remade With Rust](https://img.shields.io/badge/Remade%20With-Rust-000?logo=rust&logoColor=fff)](https://github.com/remade-with-rust)
[![By Mata Network](https://img.shields.io/badge/by-Mata%20Network-5b2be0)](https://www.mata.network)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE-MIT)
![Platforms: Windows · macOS · Linux · Web · WASM](https://img.shields.io/badge/platforms-Windows%20%C2%B7%20macOS%20%C2%B7%20Linux%20%C2%B7%20Web%20%C2%B7%20WASM-informational)
![MSRV: 1.73](https://img.shields.io/badge/MSRV-1.73-informational)

> **thoth** is an open-source Unicode glyph, design-token, and chrome a11y
> toolkit for any Rust UI -- **semantically named constants + presentation
> pinning + optional CSS / ARIA helpers** -- pure Rust. By default it also
> installs [`rusty_alloc`](https://github.com/Remade-With-Rust/rusty_alloc)
> as the process allocator (opt out below). Application `.rs` files stay
> ASCII; glyphs never scatter as raw literals that Windows-1252 round-trips
> can mojibake. Thoth is the Egyptian god of wisdom, knowledge, writing, and
> hieroglyphs.
>
> Product nicknames: **rusty_tokens** (`thoth::tokens`), **rusty_a11y**
> (`thoth::a11y`).
>
> **rusty_expressions has split out.** The Oniguruma remake now lives at
> [crates.io/crates/rusty_expressions](https://crates.io/crates/rusty_expressions)
> ([repo](https://github.com/Remade-With-Rust/rusty_expressions)) -- ~3x faster
> than libonig and differentially gated against it. The `expressions` feature
> re-exports it as `thoth::expressions`, so existing paths keep working; new
> code should depend on that crate directly.

> **Status -- v0.3.0.** Consumers pin
> `git = "https://github.com/Remade-With-Rust/thoth.git", tag = "v0.3.0"`.
> Core is `no_std` / wasm-checked. Plans:
> [symbols](docs/plans/symbols-crate.md) |
> [tokens](docs/plans/tokens-crate.md) |
> [a11y](docs/plans/a11y-crate.md) |
> [expressions](docs/plans/expressions.md).

---

## The headline

> **One immune source of truth.** Glyphs and tokens live once, as ASCII
> source. CI rejects non-ASCII `.rs` bytes so corruption cannot land again.

| Dimension | Scattered literals | **thoth (Rust)** | Goal |
|---|:---:|:---:|:---:|
| Mojibake-proof source | every file is a site | **ASCII escapes / ASCII hex** | structural |
| Naming | raw codepoints / hex | **semantic modules** | maintain |
| Presentation (WebView) | platform-dependent | **VS15 pinned glyphs** | uniform |
| Chrome theme contract | ad-hoc CSS | **`--thoth-*` tokens + optional sheet** | uniform |
| Accessibility | bare glyph, no name | **`a11y` / `html` helpers** | opt-in |
| Allocator | system / C | **`rusty_alloc` by default** | opt-out |
| License + embedding | mixed | **MIT** | -- |

---

## Install

Not on crates.io yet. From a sibling checkout or git tag:

```toml
thoth = { git = "https://github.com/Remade-With-Rust/thoth.git", tag = "v0.3.0" }
# bring your own allocator (libraries / mata-alloc portfolios):
# thoth = { git = "...", tag = "v0.3.0", default-features = false }
# accessible HTML helpers (preferred):
# thoth = { git = "...", tag = "v0.3.0", features = ["a11y"] }
# v0.1 compat alias (enables a11y + symbols::html::labelled):
# thoth = { git = "...", tag = "v0.3.0", features = ["html"] }
# CSS :root emitter for design tokens:
# thoth = { git = "...", tag = "v0.3.0", features = ["css"] }
# Oniguruma remake: prefer the standalone crate for new code --
# rusty_expressions = "X.Y"   (crates.io)
# The in-tree module remains available:
# thoth = { git = "...", tag = "v0.3.0", default-features = false, features = ["expressions"] }
# hardened allocator (guard pages + encrypted free lists):
# thoth = { git = "...", tag = "v0.3.0", features = ["secure"] }
# local path while developing:
# thoth = { path = "../thoth" }
```

| Feature | Default | Provides |
|---------|---------|----------|
| `rusty-alloc` | **yes** | process-wide [`rusty_alloc`](https://github.com/Remade-With-Rust/rusty_alloc) |
| `secure` | no | enables `rusty-alloc` + guard pages / encrypted free lists |
| *(core)* | -- | `symbols::*`, `tokens::{color,space,type_scale,radius}` |
| `a11y` | no | `a11y::{label,live,status}` |
| `html` | no | enables `a11y`; `symbols::html::labelled` re-export |
| `css` | no | `tokens::css::root_sheet` |
| `expressions` | no | re-exports the [`rusty_expressions`](https://crates.io/crates/rusty_expressions) crate as `thoth::expressions`; does not enable `rusty-alloc` |
| `compat` | no | enables `expressions`; pure-Rust `onig_new` / `regex_t` C ABI (no libonig) |

Always on: pure-ASCII source, `no_std` core. With `default-features = false`,
thoth has **zero** required dependencies.

MSRV: **1.73**.

## Memory allocation (`rusty_alloc`) -- on by default

**thoth installs [`rusty_alloc`](https://github.com/Remade-With-Rust/rusty_alloc)
as the process-wide allocator by default.** It is a pure-Rust mimalloc-class
allocator: double free aborts instead of corrupting the heap, no C allocator
in the tree, and the same surface on Linux / macOS / Windows / `wasm32`
(no emscripten). Measured evidence vs mimalloc is instruction **parity**
(~0.99--1.01); vs glibc roughly **16% fewer instructions** on the published
workloads -- see the rusty_alloc README for what is and isn't claimed.

An unconfigured app should get the hardened allocator, so this is opt-**out**:

```toml
# Bring your own (jemalloc, mimalloc, system, mata-alloc, ...)
thoth = { git = "...", tag = "v0.3.0", default-features = false }

# Or the hardened profile
thoth = { git = "...", tag = "v0.3.0", features = ["secure"] }
```

Disabling it removes `rusty_alloc` from the dependency graph entirely.
Check what a build actually got:

```rust
assert!(thoth::rusty_alloc_enabled()); // true with defaults
```

### Writing a library? Opt out.

A program may contain exactly **one** `#[global_allocator]`, and Cargo
features are additive across the whole graph. Libraries that depend on thoth
must use `default-features = false` so they do not impose an allocator on
every downstream binary (and so they do not conflict with an app that already
chose one, e.g. via `mata-alloc`).

Sibling crates [`rusty_tokens`](https://github.com/Remade-With-Rust/rusty_tokens)
and [`rusty_a11y`](https://github.com/Remade-With-Rust/rusty_a11y) do **not**
install an allocator -- only thoth (or the app) does.

## Quick start

```rust
use thoth::symbols::{nav, status, math};
use thoth::tokens::color;

fn label_ok() -> String {
    format!("{} ready", status::OK)
}

fn theme_fg_var() -> &'static str {
    color::FG // "--thoth-color-fg"
}
```

```rust
// feature = "a11y"
use thoth::a11y::{label, status as a11y_status};
use thoth::symbols::status;

fn accessible_ok() -> String {
    label::img(status::OK, "verified")
}

fn saved_banner() -> String {
    a11y_status::announce(a11y_status::Kind::Saved)
}
```

```rust
// feature = "css"
use thoth::tokens::css;

fn inject_theme() -> String {
    css::root_sheet()
}
```

```rust
// feature = "expressions"
use thoth::expressions::{Encoding, Options, Regex, Syntax};

fn find_cat(hay: &[u8]) -> Option<(usize, usize)> {
    let re = Regex::new("ca+t", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).ok()?;
    re.search(hay).ok()?.map(|m| {
        let r = m.range();
        (r.start, r.end)
    })
}
```

```sh
cargo test
cargo test --features a11y,css,html,expressions,compat
cargo test --no-default-features --features a11y,css,html,expressions,compat
cargo check --target wasm32-unknown-unknown --no-default-features --features expressions,compat

# The regex engine's own suite (harvested Oniguruma vectors, the differential
# gates vs live libonig, the property fuzz) runs in its own repository:
#   github.com/Remade-With-Rust/rusty_expressions

# Consumer CI -- fail the build on non-ASCII .rs bytes
bash scripts/check-ascii-rs.sh src crates
# Windows:
powershell -File scripts/check-ascii-rs.ps1 src crates
```

## Features

- **rusty_alloc** -- process allocator on by default; `secure` for hardening.
- **Status** -- ok / fail / warn / timer / alarm / live / play / stop.
- **Nav** -- arrows, hooks, branch, collapse (VS15 on triangles).
- **Structure** -- horizontal rule, tree tee / corner.
- **Math** -- gte / lte / approx / times.
- **List** -- bullet / middot.
- **Tokens (rusty_tokens)** -- color / space / type_scale / radius names + defaults.
- **CSS** -- `:root` sheet emitter (`css` feature).
- **A11y (rusty_a11y)** -- labelled glyphs, live regions, status announcements.
- **Expressions** -- `thoth::expressions` re-exports
  [`rusty_expressions`](https://crates.io/crates/rusty_expressions), the
  Oniguruma remake: match-equivalent to Oniguruma 6.9.10 against live libonig
  and **~3x faster than it** (`ours/onig` 0.32, 23/23 benchmark cases ours).
  New code should depend on that crate directly.
- **HTML** -- v0.1 compat re-export of `a11y::label::img`.
- **Guards** -- ASCII source self-test; consumer grep scripts.

### Capability table

| Capability | Status |
|---|---|
| `\u{...}` glyph constants (mojibake-proof) | done |
| Semantic symbol modules | done |
| VS15 presentation pinning | done |
| Design tokens (`tokens::*`) | done v0.2 |
| CSS `:root` emitter | done feature `css` |
| `a11y` label / live / status | done feature `a11y` |
| `html::labelled` compat re-export | done feature `html` |
| `rusty_alloc` default + opt-out | done v0.3 |
| ASCII self-test | done |
| Consumer CI scripts | done |
| First consumer for tokens/a11y | done Phase 1 |
| Oniguruma remake | **split out** to [rusty_expressions](https://crates.io/crates/rusty_expressions); `expressions` feature re-exports it |
| crates.io | later |

## Architecture

```text
┌──────────────────────────────────────────────────────────────┐
│  thoth                                                       │
│                                                              │
│  rusty_alloc     -- global allocator [default]            ✅ │
│  symbols::*      -- glyphs                                ✅ │
│  symbols::html   -- labelled; re-exports a11y             ✅ │
│  tokens::*       -- design tokens (rusty_tokens)          ✅ │
│  tokens::css     -- :root sheet [feature css]             ✅ │
│  a11y::*         -- chrome a11y (rusty_a11y) [a11y]       ✅ │
│  expressions::*  -- re-export of rusty_expressions [split] ✅ │
└──────────────────────────────────────────────────────────────┘
```

Plans: [symbols](docs/plans/symbols-crate.md) |
[tokens](docs/plans/tokens-crate.md) |
[a11y](docs/plans/a11y-crate.md) |
[expressions](docs/plans/expressions.md).

Northern star for glyphs: [ratatui symbols](https://github.com/ratatui/ratatui)
(semantic naming, grouped sets, flat consts) -- without terminal-first APIs or
literal glyph source bytes.

## Platform support

| Platform | Status |
|---|---|
| Windows | yes |
| macOS | yes |
| Linux | yes |
| Web (Dioxus / browsers) | yes |
| WASM (`wasm32-unknown-unknown`) | yes (`no_std` core; `a11y` / `html` / `css` need `alloc`; default `rusty_alloc` covers the heap) |

No OS APIs beyond what `rusty_alloc` uses for the heap. The same glyph/token
constants render under WebView2, WKWebView, and wasm UIs; VS15 pinning keeps
glyph presentation consistent across those hosts.

## Remade With Rust

**Remade With Rust** ([Mata Network](https://www.mata.network)) rebuilds essential
tooling in Rust -- memory safety, predictable performance, permissive license.

-> **[github.com/remade-with-rust](https://github.com/remade-with-rust)**

## License

MIT -- [LICENSE-MIT](LICENSE-MIT).

## Trademark

"Remade With Rust", "Mata", and "Mata Network" are marks of Mata Network.
