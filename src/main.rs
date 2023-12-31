use std::env;

mod processing;

fn main() {
    let args: Vec<String> = env::args().collect();
    processing::deserialize(&args[1]).unwrap()
}
