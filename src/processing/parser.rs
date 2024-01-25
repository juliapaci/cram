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

use crate::processing::lexer::*;
use std::collections::VecDeque;

// used to evaluate expressions involving increment and decrement
fn parse_expr(tokens: &mut VecDeque<Lexeme>) -> Option<node::Expr> {
    let mut expr: node::Expr = Default::default();

    if matches!(tokens.front(), Some(lexeme)
                if matches!(lexeme, Lexeme::Token(token)
                            if *token != Token::Zero)) {
        return None;
    }

    while let Some(lexeme) = tokens.pop_front() {
        if lexeme == Lexeme::Token(Token::LineBreak) {
            break;
        }

        expr.value += (lexeme == Lexeme::Token(Token::Increment)) as isize;
        expr.value -= (lexeme == Lexeme::Token(Token::Decrement)) as isize;
    }

    Some(expr)
}

pub fn parse(mut tokens: VecDeque<Lexeme>) -> Result<node::Program, String> {
    let mut program: node::Program = Default::default();

    while let Some(lexeme) = tokens.pop_front() {
        match lexeme {
            Lexeme::Token(Token::Zero) => {},
            Lexeme::Token(Token::Increment) => {},
            Lexeme::Token(Token::Decrement) => {},
            Lexeme::Token(Token::Access) => {},
            Lexeme::Token(Token::Repeat) => {},
            Lexeme::Token(Token::Quote) => {
                program.expr.push(match parse_expr(&mut tokens) {
                    Some(expr) => expr.kind,
                    None => return Err(format!("failed parsing {:?}", lexeme))
                })
            },
            Lexeme::Token(Token::LineBreak) => {},
            Lexeme::Token(Token::Variable) => {},

            _ => {}
        }
    }

    Ok(program)
}
