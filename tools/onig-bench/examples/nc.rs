use thoth::expressions::{Encoding, Options, Regex, Syntax};
fn main() {
    let n: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let pat = format!("{}a{}", "[".repeat(n), "]".repeat(n));
    eprintln!("nested-class n={n}");
    match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
        Ok(re) => println!("COMPILED search -> {:?}", re.search(b"a").map(|v| v.map(|m| m.range()))),
        Err(e) => println!("compile -> Err {:?}", e.kind),
    }
}
