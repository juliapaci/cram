pub mod node {
    use crate::processing::lexer;

    // root node
    #[derive(Default)]
    pub struct Program {
        pub expr: Vec<Expression>
    }

    // expression node
    #[derive(Default)]
    pub struct Expr {
        pub kind: Expression,
        pub value: isize
    }

    // expression
    pub enum Expression {
        Token(lexer::Token),    // key
        Identifier              // dynamic token like a variable
    }

    impl Default for Expression {
        fn default() -> Self {
            Self::Token(Default::default())
        }
    }
}

use crate::processing::lexer;
use std::collections::VecDeque;

// used to evaluate expressions involving increment and decrement
fn parse_expr(tokens: &mut VecDeque<lexer::Token>) -> Option<node::Expr> {
    let mut expr: node::Expr = Default::default();

    if let Some(token) = tokens.front() {
        if *token != lexer::Token::Zero {
            return None;
        }
    }

    while let Some(token) = tokens.pop_front() {
        if token == lexer::Token::LineBreak {
            break;
        }

        expr.value += (token == lexer::Token::Increment) as isize;
        expr.value -= (token == lexer::Token::Decrement) as isize;
    }

    Some(expr)
}

pub fn parse(mut tokens: VecDeque<lexer::Token>) -> Result<node::Program, String> {
    let mut program: node::Program = Default::default();

    while let Some(token) = tokens.pop_front() {
        match token {
            lexer::Token::Zero => {},
            lexer::Token::Increment => {},
            lexer::Token::Decrement => {},
            lexer::Token::Access => {},
            lexer::Token::Repeat => {},
            lexer::Token::Quote => {
                program.expr.push(match parse_expr(&mut tokens) {
                    Some(expr) => expr.kind,
                    None => return Err(format!("failed parsing {:?}", token))
                })
            },
            lexer::Token::LineBreak => {},
            lexer::Token::Variable => {},
        }
    }

    Ok(program)
}
