use crate::processing::parser;
use std::{io, fs};

// outputes the assembly from the ast to a file
// propogates fs::write fails
pub fn generate(program: &parser::node::Program, out: &str) -> io::Result<()> {
    let mut asm: String = Default::default();
    asm += "global _start\n";
    asm += "\t_start:\n";

    asm += "\tmov rax, 60\n";
    asm += &format!("\tmov rdi, {}\n", 1);
    asm += "\tsyscall";

    fs::write(out, asm)?;

    println!("outputted assembly to {out}");

    Ok(())
}
