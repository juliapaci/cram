mod processing {
    pub mod lexer;
    pub mod parser;
}

use std::env;

use processing::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Input the key file and the source file as arguments");
        return;
    }

    let tokens = lexer::deserialize(&args[1], &args[2]).unwrap();
    parser::parse(tokens.into()).unwrap();
}
