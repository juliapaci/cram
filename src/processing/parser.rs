use crate::processing::lexer::*;
use std::collections::VecDeque;

pub mod node {
    #[derive(Default, Debug)]
    pub struct Program {
        pub statements: Vec<Statement>
    }

    #[derive(Debug)]
    pub struct Statement {
        pub expressions: Vec<Expression>
    }

    #[derive(Debug)]
    pub struct Expression {
        pub value: isize
    }

    // scopes
    #[derive(Debug, Default)]
    pub enum ScopeType {
        Function,
        If,
        Loop,
        #[default]
        Local
    }

    #[derive(Debug, Default)]
    pub struct Scope {
        pub kind: ScopeType,
        pub condition: Option<Expression>,
        pub body: Program
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
            Some(Lexeme::Token(Token::Zero)) => Some(node::Expression{value: self.eval_lit()}),

            // Lexeme::Identifier(id) =>

            _ => None
        }
    }

    fn parse_scope(&mut self) -> Option<node::Scope> {
        let mut scope: node::Scope = Default::default();

        scope.kind = match self.tokens.pop_front() {
            Some(Lexeme::Token(Token::ScopeStart)) => {
                let init = self.tokens.pop_front();

                match init {
                    Some(Lexeme::Token(Token::Access)) => node::ScopeType::Function,
                    Some(Lexeme::Token(Token::Repeat)) => node::ScopeType::Loop,
                    // TODO: if statement
                    _ => return None
                }
            },
            _ => return None
        };

        Some(scope)
    }
}

pub fn parse(mut tokens: VecDeque<Lexeme>) -> Result<node::Program, String> {
    let mut parser = Parser {
        tokens
    };
    let mut program: node::Program = Default::default();

    while let Some(lexeme) = parser.tokens.pop_front() {
            match lexeme {
                Lexeme::Token(Token::Zero) => {}
                Lexeme::Token(Token::Increment) => {}
                Lexeme::Token(Token::Decrement) => {}
                Lexeme::Token(Token::Access) => {}
                Lexeme::Token(Token::Repeat) => {}
                Lexeme::Token(Token::Quote) => {}
                Lexeme::Token(Token::LineBreak) => {}
                Lexeme::Token(Token::Variable) => unreachable!(),
                Lexeme::Token(Token::ScopeStart) => parser.parse_scope()
                Lexeme::Token(Token::ScopeEnd) => unreachable!(),

                Lexeme::Identifier(id) => {}
            }
    }

    Ok(program)
}
