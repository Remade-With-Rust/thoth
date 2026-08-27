use thoth::expressions::{Encoding, Options, Regex, Syntax};
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kind = a[0].clone();
    let n: usize = a[1].parse().unwrap();
    let pat = match kind.as_str() {
        "group"   => format!("{}a{}", "(".repeat(n), ")".repeat(n)),
        "noncapt" => format!("{}a{}", "(?:".repeat(n), ")".repeat(n)),
        "alt"     => (0..n).map(|_| "x").collect::<Vec<_>>().join("|"),
        "star"    => format!("{}a*{}", "(".repeat(n), ")*".repeat(n)),
        "look"    => format!("{}a{}", "(?=".repeat(n), ")".repeat(n)),
        "atomic"  => format!("{}a{}", "(?>".repeat(n), ")".repeat(n)),
        "class"   => format!("[{}]", "a-z".repeat(n)),
        _ => panic!("kind"),
    };
    eprintln!("{kind} n={n} patlen={}", pat.len());
    match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
        Ok(re) => {
            println!("COMPILED");
            match re.search(b"a") {
                Ok(v) => println!("search -> {:?}", v.map(|m| m.range())),
                Err(e) => println!("search -> Err {:?}", e.kind),
            }
        }
        Err(e) => println!("compile -> Err {:?}", e.kind),
    }
}
