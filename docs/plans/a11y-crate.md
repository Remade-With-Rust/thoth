# Plan: `thoth::a11y` (rusty_a11y) -- accessibility helpers for UI chrome

- **Status:** Phase 0 done (v0.2.0 tagged); Phase 1 consumer wired (maestro-console + faucet-gui)
- **Repo:** https://github.com/Remade-With-Rust/thoth
- **Created:** 2026-08-11
- **Updated:** 2026-08-11
- **Owner:** unassigned
- **Product nickname:** rusty_a11y (module path stays `thoth::a11y`)
- **Northern star:** WAI-ARIA patterns for chrome (labelled graphics, live regions, status)
- **README format:** [`rusty_dds`](https://github.com/Remade-With-Rust/rusty_dds)

---

## 1. Problem

Glyphs and status text in WebView / Dioxus UIs are often bare characters with no
accessible name. Screen readers announce nothing useful for "checkmark" chrome,
and sync/saved/offline updates are silent.

`thoth::symbols::html::labelled` (v0.1, feature `html`) started this. `a11y`
deepens it into a small, typed set of HTML string builders -- still zero DOM
deps, still opt-in `alloc`.

## 2. Goals / non-goals

### Goals

- `a11y::label` -- glyph + aria-label (and related control labelling helpers).
- `a11y::live` -- polite / assertive live-region HTML snippets.
- `a11y::status` -- saved / syncing / offline / error announcements.
- Escape quotes in labels; emit only `role` / `aria-*` markup.
- Feature `a11y`; `html` becomes a compat layer that enables `a11y` and
  re-exports `label::img` as `symbols::html::labelled`.
- Prefer `thoth::a11y` in new code.

### Non-goals

- Full WCAG auditor / screen-reader emulation
- Document-engine / canvas a11y
- i18n (English strings for status kinds in v0.2)
- Separate published crate named `rusty_a11y` (nickname only until a later split)

## 3. Architecture

```text
thoth::a11y
  label   -- img(glyph, aria_label) and related
  live    -- polite / assertive regions
  status  -- Kind + announce()

thoth::symbols::html::labelled  -- re-export of a11y::label::img  [feature html]
```

## 4. Capability backlog (phased)

### Phase 0 -- Scaffold

- [x] `a11y::{label,live,status}`
- [x] `html = ["a11y"]` + re-export compat
- [x] Escape + announcement tests
- [x] Plan doc + README rows
- [x] Tag **v0.2.0**

### Phase 1 -- First consumer

- [x] 2-3 live/status call sites in one real UI

### Phase 2 -- Harden

- [ ] Expand helpers only from real call sites
- [ ] crates.io when stable

## 5. Design decisions (locked for v0.2)

```rust
a11y::label::img(status::OK, "verified");
a11y::live::polite("Syncing");
a11y::status::announce(a11y::status::Kind::Saved);
```

Requires `alloc`. No JS, no DOM crate.

## 6. Migration doctrine

New chrome uses `a11y`; existing `html::labelled` callers keep working via re-export.

## 7. First step

Scaffold modules, wire `html` compat, land with `tokens` under the v0.2.0 tag.
