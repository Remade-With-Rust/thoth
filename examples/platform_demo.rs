//! Cross-platform smoke demo for `thoth::tokens` + `thoth::a11y`.
//!
//! ```sh
//! cargo run --example platform_demo --features a11y,css
//! ```
//!
//! Writes `target/thoth-platform-demo.html` you can open in any browser (web).
//! The same example binary runs on Windows / macOS / Linux hosts.

use std::env;
use std::fs;
use std::path::PathBuf;

use thoth::a11y::{label, live, status};
use thoth::symbols::{nav, status as glyph};
use thoth::tokens::{color, css, space};

fn main() {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    println!("thoth platform demo");
    println!("  host: {os}/{arch}");
    println!("  tokens color FG = {} -> {}", color::FG, color::FG_VALUE);
    println!("  tokens space MD = {} -> {}", space::MD, space::MD_VALUE);

    let sheet = css::root_sheet();
    assert!(
        sheet.contains(color::FG) && sheet.contains(color::FG_VALUE),
        "root_sheet must emit color tokens"
    );
    println!("  css::root_sheet: {} bytes, ok", sheet.len());

    let ok = label::img(glyph::OK, "verified");
    let flow = label::img(nav::RIGHT, "next step");
    let syncing = live::polite("Syncing");
    let saved = status::announce(status::Kind::Saved);
    let err = status::announce_error("Could not reach server");

    assert!(ok.contains("role=\"img\"") && ok.contains("aria-label=\"verified\""));
    assert!(syncing.contains("aria-live=\"polite\""));
    assert!(saved.contains("Saved"));
    assert!(err.contains("aria-live=\"assertive\""));
    println!("  a11y::label::img: ok");
    println!("  a11y::live::polite: ok");
    println!("  a11y::status::announce: ok");

    let out = demo_html(&sheet, &ok, &flow, &syncing, &saved, &err, os, arch);
    let path = out_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, out).expect("write demo html");
    println!("  wrote {}", path.display());
    println!("open that file in a browser to validate web rendering.");
}

fn out_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/thoth-platform-demo.html")
}

fn demo_html(
    sheet: &str,
    ok: &str,
    flow: &str,
    syncing: &str,
    saved: &str,
    err: &str,
    os: &str,
    arch: &str,
) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>thoth tokens + a11y demo</title>
<style>
{sheet}
body {{
  margin: 0;
  font-family: system-ui, Segoe UI, sans-serif;
  color: var(--thoth-color-fg);
  background: var(--thoth-color-bg);
  padding: var(--thoth-space-lg);
  line-height: var(--thoth-type-leading-body);
}}
.card {{
  background: var(--thoth-color-surface);
  border: 1px solid var(--thoth-color-border);
  border-radius: var(--thoth-radius-md);
  padding: var(--thoth-space-md);
  max-width: 40rem;
}}
h1 {{ font-size: var(--thoth-type-size-title); margin-top: 0; }}
.muted {{ color: var(--thoth-color-muted); }}
.row {{ display: flex; gap: var(--thoth-space-sm); align-items: center; margin: var(--thoth-space-sm) 0; }}
.accent {{ color: var(--thoth-color-accent); }}
.ok {{ color: var(--thoth-color-success); }}
.danger {{ color: var(--thoth-color-danger); }}
</style>
</head>
<body>
  <div class="card">
    <h1>thoth demo</h1>
    <p class="muted">tokens (CSS vars) + a11y (ARIA markup). Host build: <strong>{os}/{arch}</strong></p>
    <div class="row ok">{ok} <span>labelled glyph</span></div>
    <div class="row accent">{flow} <span>nav glyph</span></div>
    <div class="row">{syncing}</div>
    <div class="row">{saved}</div>
    <div class="row danger">{err}</div>
    <p class="muted">If colors match the token defaults and a screen reader can announce the live regions, tokens + a11y are working in the browser.</p>
  </div>
</body>
</html>
"##,
        sheet = sheet,
        ok = ok,
        flow = flow,
        syncing = syncing,
        saved = saved,
        err = err,
        os = os,
        arch = arch,
    )
}
