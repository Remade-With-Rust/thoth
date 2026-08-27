//! First consumer: grep-like op over thoth::expressions (callable from tests).
//!
//! ```sh
//! cargo run --example expressions_demo --features expressions -- "ca+t" "one cat two caaat"
//! ```

use std::env;

use thoth::expressions::{find_all_str, format_matches, is_match_str};

fn main() {
    let mut args = env::args().skip(1);
    let pattern = args.next().unwrap_or_else(|| String::from("ca+t"));
    let hay = args.next().unwrap_or_else(|| String::from("one cat two caaat"));
    match run(&pattern, &hay) {
        Ok(out) => print!("{out}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn run(pattern: &str, hay: &str) -> Result<String, thoth::expressions::Error> {
    let matched = is_match_str(pattern, hay)?;
    let ranges = find_all_str(pattern, hay)?;
    let mut out = format!("match={matched} count={}\n", ranges.len());
    out.push_str(&format_matches(hay, &ranges));
    Ok(out)
}
