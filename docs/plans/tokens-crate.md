# Plan: `thoth::tokens` (rusty_tokens) -- design tokens for any Rust UI

- **Status:** Phase 0 scaffold in progress (v0.2)
- **Repo:** https://github.com/Remade-With-Rust/thoth
- **Created:** 2026-08-11
- **Updated:** 2026-08-11
- **Owner:** unassigned
- **Product nickname:** rusty_tokens (module path stays `thoth::tokens`)
- **Northern star:** CSS design-token practice (semantic roles; Open Props shape without a theme engine)
- **README format:** [`rusty_dds`](https://github.com/Remade-With-Rust/rusty_dds)

---

## 1. Problem

App chrome (toolbars, sidebars, status bars, panels) hard-codes colors, spacing,
and radii as scattered hex/rem literals. Consequences:

1. **No shared contract** across apps or between Rust and CSS.
2. **Theme drift** -- the same "accent" is three different hexes in three crates.
3. **No single place** to emit a `:root` sheet for WebView / Dioxus injection.

This is the presentation counterpart to `thoth::symbols` (glyphs): semantic names,
ASCII-safe source, optional emitter.

## 2. Goals / non-goals

### Goals

- Semantic token **names** as CSS custom-property contracts (`--thoth-color-fg`).
- Small **neutral default value set** (ASCII `#RRGGBB`, rem/spacing/radius).
- Group by role: `color`, `space`, `type_scale`, `radius`.
- Optional `css` feature: `tokens::css::root_sheet()` for `:root { ... }`.
- `no_std` core consts; ASCII self-test covers new modules.
- rusty_dds-style README capability rows.

### Non-goals

- Runtime theme switching / OS dark-mode detection
- Tailwind plugins, Dioxus components, full design systems
- Brand-locked Mata themes inside defaults
- Separate published crate named `rusty_tokens` (nickname only until a later split)

## 3. Architecture

```text
thoth::tokens
  color       -- fg / bg / accent / danger / success / muted / border
  space       -- xs ... xl
  type_scale  -- font-size / line-height steps
  radius      -- sm / md / lg
  css         -- root_sheet()  [feature = "css"]
```

## 4. Capability backlog (phased)

### Phase 0 -- Scaffold

- [x] `tokens::{color,space,type_scale,radius}` consts (names + default values)
- [x] Feature `css` + `root_sheet()`
- [x] ASCII + value tests
- [x] Plan doc + README rows
- [ ] Tag **v0.2.0** with symbols + a11y

### Phase 1 -- First consumer

- [ ] Wire one UI to inject `root_sheet()` (faucet-gui or mata-master console)

### Phase 2 -- Harden

- [ ] Expand roles only from real call-site survey
- [ ] Optional `root_sheet_dark` if a consumer needs it
- [ ] crates.io when stable

## 5. Design decisions (locked for v0.2)

```rust
pub const FG: &str = "--thoth-color-fg";
pub const FG_VALUE: &str = "#1a1a1a";
```

Defaults are a **neutral starter**; apps override via CSS. Source stays pure ASCII.

## 6. Migration doctrine

Prevention first: new chrome uses tokens; migrate on touch; no bulk theme rewrite in v0.2.

## 7. First step

Scaffold modules + `css` feature + tests; land with `a11y` under the v0.2.0 tag.
