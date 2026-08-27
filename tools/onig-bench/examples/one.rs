use std::env;
use std::time::Instant;
use thoth::expressions::{Encoding, MatchParam, Options, Regex, Syntax};
fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    let pat = a[0].clone();
    let n: usize = a[1].parse().unwrap();
    let unit = a.get(2).cloned().unwrap_or("ab".into());
    let hay = unit.repeat(n / unit.len().max(1));
    eprintln!("pat={pat:?} bytes={}", hay.len());
    let re = Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
    let t = Instant::now();
    let r = re.search_param(hay.as_bytes(), &MatchParam::default());
    let ours = match r { Ok(Some(m)) => format!("{:?}", m.range()), Ok(None) => "miss".into(), Err(e) => format!("Err {:?}", e.kind) };
    let el = t.elapsed().as_secs_f64() * 1000.0;
    println!("ours={ours}  {el:.1}ms");
}
