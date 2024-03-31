// recursive descent parser
use crate::processing::lexer::*;
use std::collections::VecDeque;

pub mod node {
    #[derive(Default, Debug)]
    pub struct Program {
        pub statements: Vec<Statement>
    }

    #[derive(Default, Debug)]
    pub struct Statement {
        pub expressions: Vec<Expression>
    }

    #[derive(Debug)]
    pub enum Expression {
        Scope(Scope),
        IntLit(isize),
        StringLit(String),
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
        pub signature: Option<Statement>,
        pub body: Program
    }
}

struct Parser<'a> {
    tokens: &'a mut VecDeque<Lexeme>
}

impl Parser<'_> {
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


    fn parse_int(&mut self) -> Option<isize> {
        match self.tokens.front() {
            Some(Lexeme::Token(Token::Zero)) => Some(self.eval_lit()),

            // Lexeme::Identifier(id) =>

            _ => None
        }
    }

    // TODO: return type Result with ScopeError type that can be error for condition, body, etc.
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

        scope.signature = self.parse_statement();

        scope.body = parse(&mut self.tokens).unwrap(); // unwrap() is fine, Err isnt possible

        Some(scope)
    }

    // TODO: maybe a parse_line()

    fn parse_statement(&mut self) -> Option<node::Statement> {
        use node::Expression::*;

        let mut statement: node::Statement = Default::default();

        // TODO: should node::Expressions be put here or should the parsing functions return them?
        // TODO: replace unwraps with proper error handling
        while let Some(lexeme) = self.tokens.pop_front() {
            statement.expressions.push(match lexeme {
                Lexeme::Token(Token::Zero)      => IntLit(self.parse_int().unwrap()),
                Lexeme::Token(Token::Increment) => unreachable!(),
                Lexeme::Token(Token::Decrement) => unreachable!(),
                Lexeme::Token(Token::Access)    => todo!(),
                Lexeme::Token(Token::Repeat)    => todo!(),
                Lexeme::Token(Token::Quote)     => todo!(),
                Lexeme::Token(Token::Variable)  => unreachable!(),
                Lexeme::Token(Token::ScopeStart)=> Scope(self.parse_scope().unwrap()),
                Lexeme::Token(Token::ScopeEnd)  => todo!(),

                Lexeme::Identifier(id)          => todo!(),

                Lexeme::Token(Token::LineBreak) => return Some(statement),
            });
        }

        None
    }
}

pub fn parse(tokens: &mut VecDeque<Lexeme>) -> Result<node::Program, String> {
    let mut parser = Parser {
        tokens
    };
    let mut program: node::Program = Default::default();

    let mut statement = parser.parse_statement();
    while statement.is_some() {
        program.statements.push(statement.unwrap());
        statement = parser.parse_statement();
    }

    Ok(program)
}
