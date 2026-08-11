//! Crate self-tests: ASCII source gate + scalar values.

use std::fs;
use std::path::PathBuf;

use thoth::symbols::{list, math, nav, status, structure, VS15};

fn src_rs_files() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read src") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn source_is_pure_ascii() {
    let mut offenders = Vec::new();
    for path in src_rs_files() {
        let bytes = fs::read(&path).expect("read");
        for (i, b) in bytes.iter().enumerate() {
            if *b > 0x7F {
                offenders.push(format!(
                    "{}: offset {i} has non-ASCII byte 0x{b:02X}",
                    path.display()
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "thoth source must stay ASCII (use \\u{{…}} escapes):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn status_scalars() {
    assert_eq!(status::OK, concat!("\u{2713}", "\u{FE0E}"));
    assert_eq!(status::PASS, "\u{2705}");
    assert_eq!(status::FAIL, "\u{2717}");
    assert_eq!(status::REJECT, "\u{274C}");
    assert_eq!(status::CROSS, "\u{2715}");
    assert_eq!(status::WARN, concat!("\u{26A0}", "\u{FE0E}"));
    assert_eq!(status::TIMER, concat!("\u{23F1}", "\u{FE0E}"));
    assert_eq!(status::PENDING, status::TIMER);
    assert_eq!(status::ALARM, concat!("\u{23F0}", "\u{FE0E}"));
    assert_eq!(status::HOURGLASS, concat!("\u{23F3}", "\u{FE0E}"));
    assert_eq!(status::LIVE, "\u{25CF}");
    assert_eq!(status::STOP, "\u{25A0}");
    assert_eq!(status::PLAY, concat!("\u{25B6}", "\u{FE0E}"));
}

#[test]
fn nav_scalars() {
    assert_eq!(nav::RIGHT, "\u{2192}");
    assert_eq!(nav::LEFT, "\u{2190}");
    assert_eq!(nav::BIDI, "\u{2194}");
    assert_eq!(nav::DOUBLE_RIGHT, "\u{21D2}");
    assert_eq!(nav::LONG_RIGHT, "\u{27F6}");
    assert_eq!(nav::NE, "\u{2197}");
    assert_eq!(nav::HOOK_RIGHT, "\u{21AA}");
    assert_eq!(nav::HOOK_LEFT, "\u{21A9}");
    assert_eq!(nav::BRANCH, "\u{21B3}");
    assert_eq!(nav::CURVE_UP, "\u{2934}");
    assert_eq!(nav::RELOAD, "\u{21BB}");
    assert_eq!(nav::COLLAPSE, concat!("\u{25B2}", "\u{FE0E}"));
}

#[test]
fn structure_math_list_scalars() {
    assert_eq!(structure::RULE_H, "\u{2500}");
    assert_eq!(structure::TREE_TEE, "\u{251C}");
    assert_eq!(structure::TREE_CORNER, "\u{2514}");
    assert_eq!(math::GTE, "\u{2265}");
    assert_eq!(math::LTE, "\u{2264}");
    assert_eq!(math::APPROX, "\u{2248}");
    assert_eq!(math::TIMES, "\u{00D7}");
    assert_eq!(math::MINUS, "\u{2212}");
    assert_eq!(math::PLUS_MINUS, "\u{00B1}");
    assert_eq!(math::DELTA, "\u{0394}");
    assert_eq!(math::MICRO, "\u{00B5}");
    assert_eq!(math::SQRT, "\u{221A}");
    assert_eq!(math::ELEMENT_OF, "\u{2208}");
    assert_eq!(list::BULLET, "\u{2022}");
    assert_eq!(list::MIDDOT, "\u{00B7}");
    assert_eq!(VS15, "\u{FE0E}");
}

#[cfg(feature = "html")]
#[test]
fn html_labelled() {
    use thoth::symbols::html;
    let s = html::labelled(status::OK, "verified");
    assert_eq!(
        s,
        format!(
            r#"<span role="img" aria-label="verified">{}</span>"#,
            status::OK
        )
    );
    let escaped = html::labelled(status::FAIL, r#"say "no""#);
    assert!(escaped.contains(r#"aria-label="say &quot;no&quot;""#));
}
