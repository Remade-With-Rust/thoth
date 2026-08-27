# Plan: rusty_expressions -- split out of thoth

- **Status:** **Done and split out.** The engine is published as
  [`rusty_expressions`](https://crates.io/crates/rusty_expressions) and
  developed at
  [Remade-With-Rust/rusty_expressions](https://github.com/Remade-With-Rust/rusty_expressions).
- **In this repo:** the `expressions` feature re-exports that crate as
  `thoth::expressions`, so paths consumers already use keep working. There is
  no engine source here any more -- one copy, in one place.
- **Canonical design doc, backlog and phase history:** `docs/design.md` in the
  crate's repository.

## Why it split

It outgrew a feature-gated module. It is ~6_500 lines of engine plus generated
Unicode 16.0 and CJK tables, with a test apparatus -- harvested Oniguruma
vectors, differential gates against live libonig, an API property fuzz -- that
is larger than the rest of thoth put together. thoth is UI chrome; a regex
engine has different consumers, a different release cadence and a different
risk profile.

## The one trap if you depend on both

`rusty_expressions` installs `rusty_alloc` as a `#[global_allocator]` under its
default feature, and so does thoth. A program may define exactly one, so thoth
takes the dependency with `default-features = false`. If you depend on both
crates directly, only one of them may bring the allocator.

Sibling plans: [symbols](symbols-crate.md), [tokens](tokens-crate.md),
[a11y](a11y-crate.md).
