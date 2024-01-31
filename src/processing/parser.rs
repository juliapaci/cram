use crate::processing::lexer::*;
use std::collections::VecDeque;

pub mod node {
    #[derive(Debug)]
    pub struct Expression {
        pub value: isize
    }

    #[derive(Debug)]
    pub struct Statement {
        pub expressions: Vec<Expression>
    }

    #[derive(Default, Debug)]
    pub struct Program {
        pub statements: Vec<Statement>
    }
}

struct Parser {
    tokens: VecDeque<Lexeme>
}

impl Parser {
    // used to evaluate integer literal expressions involving increment and decrement
    fn eval_lit(&mut self) -> isize {
        let mut value = Default::default();

        while let Some(lexeme) = self.tokens.pop_front() {
            if lexeme == Lexeme::Token(Token::LineBreak) {
                break;
            }

            value += (lexeme == Lexeme::Token(Token::Increment)) as isize;
            value -= (lexeme == Lexeme::Token(Token::Decrement)) as isize;
        }

        value
    }


    fn parse_expr(&mut self) -> Option<node::Expression> {
        match self.tokens.front() {
            Some(Lexeme::Token(token)) => {
                if *token != Token::Zero  {
                    return None
                }
                Some(node::Expression{value: self.eval_lit()})
            }

            // Lexeme::Identifier(id) =>

            _ => None
        }
    }
}

pub fn parse(mut tokens: VecDeque<Lexeme>) -> Result<node::Program, String> {
    let mut parser = Parser {
        tokens
    };
    let mut program: node::Program = Default::default();

    while let Some(lexeme) = parser.tokens.pop_front() {
            match lexeme {
                Lexeme::Token(Token::Zero) => {},
                Lexeme::Token(Token::Increment) => {},
                Lexeme::Token(Token::Decrement) => {},
                Lexeme::Token(Token::Access) => {},
                Lexeme::Token(Token::Repeat) => {},
                Lexeme::Token(Token::Quote) => {},
                Lexeme::Token(Token::LineBreak) => {},

                Lexeme::Identifier(id) => {}

                Lexeme::Token(Token::Variable) => {}, // not possible
            }
    }

    Ok(program)
}
