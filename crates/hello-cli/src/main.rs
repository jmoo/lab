use std::env;

use greeting::greet;

fn main() {
    let name = env::args().nth(1).unwrap_or_else(|| "world".to_string());
    println!("{}", greet(&name));
}
