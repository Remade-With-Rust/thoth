//! Required-literal analyzer report.
//!
//! cargo run --release --manifest-path tools/onig-bench/Cargo.toml --example reqlit
//!
//! Shows, per pattern, the byte sequence every match must contain, how far it
//! sits from the match start, and whether it is anchored to a preceding class
//! run (which is what makes an unbounded distance usable).
use thoth::expressions::{Encoding, Options, Regex, Syntax};

const PATS: &[&str] = &[
    "fox",
    "[0-9]+",
    r"\w+",
    "fox|dog|cat",
    "INFO|WARN|ERROR|DEBUG",
    r"(?m)^2026",
    r"(\d{4})-(\d{2})-(\d{2})",
    r"(\w+)=(\w+)",
    r"(?<k>\w+)=(?<v>\w+)",
    r"[\w.]+@[\w.]+\.\w+",
    r"https?://[\w./?=&-]+",
    r"\d+\.\d+\.\d+\.\d+",
    r"(\w+) \1",
    r"\d+(?= ms)",
    r"(?<=status=)\d+",
    r"(?>\w+)=",
    r"\d++ms",
    "(?i)THE QUICK",
    "(?i)[a-z]+ing",
    r"\bcat\b",
    "a.*b",
    "(?:ab)+cd",
];

fn main() {
    println!(
        "{:<26} {:<14} {:>8} {:>8}  {}",
        "pattern", "required", "min_d", "max_d", "run-anchored"
    );
    println!("{}", "-".repeat(78));
    let mut found = 0;
    for pat in PATS {
        let re = match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA)
        {
            Ok(r) => r,
            Err(e) => {
                println!("{:<26} compile-err {}", pat, e);
                continue;
            }
        };
        match re.required_literal() {
            Some(r) => {
                found += 1;
                let text: String = r
                    .bytes
                    .iter()
                    .map(|b| {
                        if b.is_ascii_graphic() {
                            (*b as char).to_string()
                        } else {
                            format!("\\x{b:02x}")
                        }
                    })
                    .collect();
                println!(
                    "{:<26} {:<14} {:>8} {:>8}  {}",
                    pat,
                    format!("{text:?}"),
                    r.min_dist,
                    r.max_dist
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "inf".into()),
                    if r.run_class.is_some() { "yes" } else { "-" }
                );
            }
            None => println!(
                "{:<26} {:<14} {:>8} {:>8}  -",
                pat, "(none)", "-", "-"
            ),
        }
    }
    println!("\n{found}/{} patterns carry a usable required literal", PATS.len());
}
