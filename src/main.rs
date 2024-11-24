use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        println!("Input the key file, source file, and output file paths as respective arguments");
        return;
    }

    // lexer
    let tokens = lexer::deserialize(&args[1], &args[2]).unwrap();
    println!("{:?} ({})", tokens, tokens.len());

    // parser
    let program = match parser::parse(&mut tokens.into()) {
        Ok(p) => p,
        Err(e) => {
            println!("{e}");
            return;
        }
    };
    println!("Finished parsing:");
    println!("{program:?}");

    // codegen
    let out_name = format!(
        "out/{}",
        Path::new(&args[3]).file_stem().unwrap().to_str().unwrap()
    );
    codegen::generate(&program, &format!("{}.s", out_name)).expect("failed to asm write to file");

    Command::new("nasm") // assemble
        .args(["-felf64", &format!("{}.s", out_name)])
        .output()
        .expect("nasm failed");
    Command::new("ld") // link
        .arg(format!("{}.o", out_name))
        .args(["-o", &args[3]])
        .output()
        .expect("ld failed");
}
