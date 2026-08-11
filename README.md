# thoth

[![Remade With Rust](https://img.shields.io/badge/Remade%20With-Rust-000?logo=rust&logoColor=fff)](https://github.com/remade-with-rust)
[![By Mata Network](https://img.shields.io/badge/by-Mata%20Network-5b2be0)](https://www.mata.network)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE-MIT)
![Platforms: Windows · macOS · Linux · Web · WASM](https://img.shields.io/badge/platforms-Windows%20%C2%B7%20macOS%20%C2%B7%20Linux%20%C2%B7%20Web%20%C2%B7%20WASM-informational)
![MSRV: 1.73](https://img.shields.io/badge/MSRV-1.73-informational)

> **thoth** is a shared Unicode glyph toolkit for House Rust apps —
> **semantically named constants + presentation pinning + optional HTML
> a11y helpers** — pure Rust, zero dependencies, so Dioxus / WebView UIs
> never scatter raw glyph literals that Windows-1252 round-trips can
> mojibake. Thoth is the Egyptian god of knowledge and hieroglyphs.

> **Status — pre-1.0, Phase 2 substantially complete.** Consumers:
> faucet (Phase 1), Dial, comet, mata-maestro; mata-master has the
> workspace dep + Coding Requirements rule (migrate on touch).
> Core crate is `no_std` and checked on `wasm32-unknown-unknown`.
> **Next:** tag `v0.1.0`, optional mata-master package-by-package
> codemod (Phase 3). Plan:
> [docs/plans/symbols-crate.md](docs/plans/symbols-crate.md).

---

## The headline

> **One immune source of truth.** Glyphs live once, as ASCII `\u{…}`
> escapes. Application `.rs` files stay pure ASCII; CI rejects any
> non-ASCII byte so corruption cannot land again and new symbols must
> come through this crate.

| Dimension | Scattered literals | **thoth (Rust)** | Goal |
|---|:---:|:---:|:---:|
| Mojibake-proof source | every file is a site | **ASCII escapes only** | structural |
| Naming | raw codepoints | **semantic modules** | maintain |
| Presentation (WebView2 / WKWebView) | platform-dependent | **VS15 pinned** | uniform |
| Accessibility | bare glyph, no name | **`html` labelled spans** | opt-in |
| Dependencies | — | **none** | maintain |
| License + embedding | mixed | **MIT** | — |

---

## Install

Not on crates.io yet. From a sibling checkout or git tag:

```toml
thoth = { git = "https://github.com/Remade-With-Rust/thoth.git", tag = "v0.1.0" }
# accessible HTML helper:
# thoth = { git = "https://github.com/Remade-With-Rust/thoth.git", tag = "v0.1.0", features = ["html"] }
# local path while developing:
# thoth = { path = "../thoth" }
```

| Feature | Default | Provides |
|---------|---------|----------|
| *(none)* | — | `symbols::{status,nav,structure,math,list}` |
| `html` | no | `symbols::html::labelled` |

Always on: pure-ASCII constants, `no_std`, zero deps.

MSRV: **1.73**.

## Quick start

```rust
use thoth::symbols::{nav, status, math};

fn label_ok() -> String {
    format!("{} ready", status::OK)
}

fn arrow_flow() -> String {
    format!("A {} B", nav::RIGHT)
}

fn warmup() -> String {
    format!("n {} 1", math::GTE)
}
```

```rust
// feature = "html"
use thoth::symbols::{html, status};

fn accessible_ok() -> String {
    html::labelled(status::OK, "verified")
    // -> <span role="img" aria-label="verified">…</span>
}
```

```sh
cargo test
cargo test --features html

# Consumer CI — fail the build on non-ASCII .rs bytes
bash scripts/check-ascii-rs.sh src crates
# Windows:
powershell -File scripts/check-ascii-rs.ps1 src crates
```

## Features

- **Status** — ok / fail / warn / timer / alarm / live / play / stop.
- **Nav** — arrows, hooks, branch, collapse (VS15 on triangles).
- **Structure** — horizontal rule, tree tee / corner.
- **Math** — gte / lte / approx / times.
- **List** — bullet / middot.
- **HTML** — labelled `role="img"` spans (`html` feature).
- **Guards** — ASCII source self-test; consumer grep scripts.

### Capability table

| Capability | Status |
|---|---|
| `\u{…}` constants (mojibake-proof) | ✅ |
| Semantic modules (`status` / `nav` / …) | ✅ |
| VS15 presentation pinning | ✅ |
| `html::labelled` | ✅ feature |
| ASCII self-test | ✅ |
| Consumer CI scripts | ✅ |
| Faucet wired as first consumer | ✅ Phase 1 |
| Dial / comet / mata-maestro consumers | ✅ Phase 2 |
| mata-master workspace dep + on-touch rule | ✅ Phase 2 |
| Bulk migration of mata-master existing glyphs | ⏳ on-touch / Phase 3 |

## Architecture

```text
┌─────────────────────────────────────────────────────────┐
│  thoth                                                  │
│                                                         │
│  symbols::status     — ok / fail / warn / timer    ✅   │
│  symbols::nav        — arrows / hooks / collapse   ✅   │
│  symbols::structure  — rules / tree lines          ✅   │
│  symbols::math       — gte / lte / approx / times  ✅   │
│  symbols::list       — bullet / middot             ✅   │
│  symbols::html       — labelled <span> [feature]   ✅   │
└─────────────────────────────────────────────────────────┘
```

Plan: [docs/plans/symbols-crate.md](docs/plans/symbols-crate.md).

Northern star: [ratatui symbols](https://github.com/ratatui/ratatui) (semantic
naming, grouped sets, flat consts) — without terminal-first APIs or literal
glyph source bytes.

## Platform support

| Platform | Status |
|---|---|
| Windows | ✅ |
| macOS | ✅ |
| Linux | ✅ |
| Web (Dioxus / browsers) | ✅ |
| WASM (`wasm32-unknown-unknown`) | ✅ (`no_std` core; `html` needs `alloc`) |

No OS APIs, no `std` requirement on the default feature set. The same
constants render under WebView2, WKWebView, and wasm UIs; VS15 pinning
keeps presentation consistent across those hosts.

## Remade With Rust

**Remade With Rust** ([Mata Network](https://www.mata.network)) rebuilds essential
tooling in Rust — memory safety, predictable performance, permissive license.

→ **[github.com/remade-with-rust](https://github.com/remade-with-rust)**

## License

MIT — [LICENSE-MIT](LICENSE-MIT).

## Trademark

"Remade With Rust", "Mata", and "Mata Network" are marks of Mata Network.
