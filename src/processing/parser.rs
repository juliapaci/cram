mod node {
    use crate::processing::lexer;

    // root node
    pub struct Program {
        expression: Expr
    }

    impl Program {
        pub fn new() -> Self {
            Self {
                expression: Default::default()
            }
        }
    }

    // expression node
    #[derive(Default)]
    pub struct Expr {
        pub literal: isize
    }
}

use crate::processing::lexer;
use std::collections::VecDeque;

fn parse_expr(tokens: &mut VecDeque<lexer::Token>) -> Option<node::Expr> {
    let mut expr: node::Expr = Default::default();

    if let Some(token) = tokens.front() {
        if *token != lexer::Token::Zero {
            return None;
        }
    }

    while let Some(token) = tokens.pop_front() {
        expr.literal += (token == lexer::Token::Increment) as isize;
        expr.literal -= (token == lexer::Token::Decrement) as isize;
    }

    Some(expr)
}

pub fn parse(mut tokens: VecDeque<lexer::Token>) -> Result<node::Program, String> {
    let program = node::Program::new();

    while let Some(token) = tokens.pop_front() {
        match token {
            lexer::Token::Zero => {},
            lexer::Token::Increment => {},
            lexer::Token::Decrement => {},
            lexer::Token::Access => {},
            lexer::Token::Repeat => {},
            lexer::Token::Quote => {
                println!("{}", parse_expr(&mut tokens).unwrap().literal);
            },
            lexer::Token::LineBreak => {},
        }
    }

    Ok(program)
}
