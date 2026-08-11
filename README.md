# thoth

[![Remade With Rust](https://img.shields.io/badge/Remade%20With-Rust-000?logo=rust&logoColor=fff)](https://github.com/remade-with-rust)
[![By Mata Network](https://img.shields.io/badge/by-Mata%20Network-5b2be0)](https://www.mata.network)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE-MIT)
![Platforms: Windows · macOS · Linux · Web · WASM](https://img.shields.io/badge/platforms-Windows%20%C2%B7%20macOS%20%C2%B7%20Linux%20%C2%B7%20Web%20%C2%B7%20WASM-informational)
![MSRV: 1.73](https://img.shields.io/badge/MSRV-1.73-informational)

> **thoth** is an open-source Unicode glyph, design-token, and chrome a11y
> toolkit for any Rust UI -- **semantically named constants + presentation
> pinning + optional CSS / ARIA helpers** -- pure Rust, zero dependencies.
> Application `.rs` files stay ASCII; glyphs never scatter as raw literals
> that Windows-1252 round-trips can mojibake. Thoth is the Egyptian god of
> knowledge and hieroglyphs.
>
> Product nicknames: **rusty_tokens** (`thoth::tokens`), **rusty_a11y**
> (`thoth::a11y`).

> **Status -- v0.2.0.** Consumers pin
> `git = "https://github.com/Remade-With-Rust/thoth.git", tag = "v0.2.0"`.
> Core is `no_std` / wasm-checked. Plans:
> [symbols](docs/plans/symbols-crate.md) |
> [tokens](docs/plans/tokens-crate.md) |
> [a11y](docs/plans/a11y-crate.md).

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
| Dependencies | -- | **none** | maintain |
| License + embedding | mixed | **MIT** | -- |

---

## Install

Not on crates.io yet. From a sibling checkout or git tag:

```toml
thoth = { git = "https://github.com/Remade-With-Rust/thoth.git", tag = "v0.2.0" }
# accessible HTML helpers (preferred):
# thoth = { git = "...", tag = "v0.2.0", features = ["a11y"] }
# v0.1 compat alias (enables a11y + symbols::html::labelled):
# thoth = { git = "...", tag = "v0.2.0", features = ["html"] }
# CSS :root emitter for design tokens:
# thoth = { git = "...", tag = "v0.2.0", features = ["css"] }
# local path while developing:
# thoth = { path = "../thoth" }
```

| Feature | Default | Provides |
|---------|---------|----------|
| *(none)* | -- | `symbols::*`, `tokens::{color,space,type_scale,radius}` |
| `a11y` | no | `a11y::{label,live,status}` |
| `html` | no | enables `a11y`; `symbols::html::labelled` re-export |
| `css` | no | `tokens::css::root_sheet` |

Always on: pure-ASCII source, `no_std` core, zero deps.

MSRV: **1.73**.

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

```sh
cargo test
cargo test --features a11y,css,html

# Consumer CI -- fail the build on non-ASCII .rs bytes
bash scripts/check-ascii-rs.sh src crates
# Windows:
powershell -File scripts/check-ascii-rs.ps1 src crates
```

## Features

- **Status** -- ok / fail / warn / timer / alarm / live / play / stop.
- **Nav** -- arrows, hooks, branch, collapse (VS15 on triangles).
- **Structure** -- horizontal rule, tree tee / corner.
- **Math** -- gte / lte / approx / times.
- **List** -- bullet / middot.
- **Tokens (rusty_tokens)** -- color / space / type_scale / radius names + defaults.
- **CSS** -- `:root` sheet emitter (`css` feature).
- **A11y (rusty_a11y)** -- labelled glyphs, live regions, status announcements.
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
| ASCII self-test | done |
| Consumer CI scripts | done |
| First consumer for tokens/a11y | Phase 1 |
| crates.io | later |

## Architecture

```text
┌──────────────────────────────────────────────────────────────┐
│  thoth                                                       │
│                                                              │
│  symbols::*          -- glyphs                            ✅ │
│  symbols::html       -- labelled; re-exports a11y         ✅ │
│  tokens::*           -- design tokens (rusty_tokens)      ✅ │
│  tokens::css         -- :root sheet [feature css]         ✅ │
│  a11y::*             -- chrome a11y (rusty_a11y) [a11y]   ✅ │
└──────────────────────────────────────────────────────────────┘
```

Plans: [symbols](docs/plans/symbols-crate.md) |
[tokens](docs/plans/tokens-crate.md) |
[a11y](docs/plans/a11y-crate.md).

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
| WASM (`wasm32-unknown-unknown`) | yes (`no_std` core; `a11y` / `html` / `css` need `alloc`) |

No OS APIs on the default feature set. The same constants render under
WebView2, WKWebView, and wasm UIs; VS15 pinning keeps glyph presentation
consistent across those hosts.

## Remade With Rust

**Remade With Rust** ([Mata Network](https://www.mata.network)) rebuilds essential
tooling in Rust -- memory safety, predictable performance, permissive license.

-> **[github.com/remade-with-rust](https://github.com/remade-with-rust)**

## License

MIT -- [LICENSE-MIT](LICENSE-MIT).

## Trademark

"Remade With Rust", "Mata", and "Mata Network" are marks of Mata Network.
