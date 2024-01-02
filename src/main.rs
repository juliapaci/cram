use std::env;

mod processing;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Input the key file and the source file as arguments");
        return;
    }

    processing::deserialize(&args[1], &args[2]).unwrap()
}
