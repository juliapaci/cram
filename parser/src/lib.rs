// recursive descent parser
use lexer::*;

use std::collections::HashMap;

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
        ScopeEnd, // TODO: better way to find scopeEnd this is not good
        IntLit(isize),
        StringLit(String),
        Variable(usize)
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
    tokens: &'a mut Vec<Lexeme>,
    symbol_table: HashMap<usize, isize> // TODO: be generic so we dont just have integer data types
}

impl Parser<'_> {
    // used to evaluate integer literal expressions involving increment and decrement
    fn eval_lit(&mut self) -> isize {
        let mut value = Default::default();

        while let Some(lexeme) = self.tokens.pop() {
            let increment = lexeme == Lexeme::Token(Token::Increment);
            let decrement = lexeme == Lexeme::Token(Token::Decrement);

            if !increment && !decrement {
                break;
            }

            value += increment as isize;
            value -= decrement as isize;
        }

        value
    }

    fn parse_int(&mut self) -> Option<isize> {
        Some(self.eval_lit())
    }

    // TODO: return type Result with ScopeError type that can be error for condition, body, etc.
    fn parse_scope(&mut self) -> Option<node::Scope> {
        let mut scope: node::Scope = Default::default();

        scope.kind = match self.tokens.pop() {
            Some(Lexeme::Token(Token::Access)) => node::ScopeType::Function,
            Some(Lexeme::Token(Token::Repeat)) => node::ScopeType::Loop,
            // TODO: if statement
            _ => return None
        };

        scope.signature = Some(self.parse_line()?);

        scope.body = self.parse_body()?;

        Some(scope)
    }

    fn parse_quote(&mut self) -> Option<node::Expression> {
        let string = node::Expression::StringLit(self.parse_int()?.to_string());
        self.tokens.pop(); // pops the ending quote
        Some(string)
    }

    // TODO: this is duplicate code for parse()
    fn parse_body(&mut self) -> Option<node::Program> {
        let mut program: node::Program = Default::default();

        let mut line = self.parse_line();
        while !line.as_ref()?.expressions.is_empty() {
            program.statements.push(line.unwrap());

            line = self.parse_line();
            if let Some(line) = &line {
                match line.expressions.last()? {
                    node::Expression::ScopeEnd => break,
                    _ => continue
                }
            }
        }

        Some(program)
    }

    // adds a variable to the symbol_table
    fn add_var(&mut self) -> Option<node::Expression> {
        let id = self.tokens.pop();
        if let Some(Lexeme::Identifier(id)) = id {
            self.symbol_table.insert(id, 0);
            return Some(node::Expression::Variable(id))
        }

        None
    }

    fn replace_var(&self, id: usize) -> Option<&isize> {
        self.symbol_table.get(&id)
    }

    fn parse_line(&mut self) -> Option<node::Statement> {
        use node::Expression::*;

        let mut statement: node::Statement = Default::default();

        // TODO: should node::Expressions be put here or should the parsing functions return them?
        // TODO: replace unwraps with proper error handling
        while let Some(lexeme) = self.tokens.pop() {
            statement.expressions.push(match lexeme {
                Lexeme::Token(Token::Zero)      => IntLit(self.parse_int()?),
                Lexeme::Token(Token::Increment) => unreachable!(),
                Lexeme::Token(Token::Decrement) => unreachable!(),
                Lexeme::Token(Token::Access)    => self.add_var()?,
                Lexeme::Token(Token::Variable)  => unreachable!(),
                Lexeme::Identifier(id)          => IntLit(*self.replace_var(id)?), // TODO: maybe part of parse_int()
                Lexeme::Token(Token::Repeat)    => unreachable!(),
                Lexeme::Token(Token::Quote)     => self.parse_quote()?,
                Lexeme::Token(Token::ScopeStart)=> Scope(self.parse_scope()?),
                Lexeme::Token(Token::ScopeEnd)  => return Some(statement),

                Lexeme::Token(Token::LineBreak) => return Some(statement)
            });
        }

        None
    }
}

pub fn parse(tokens: &mut Vec<Lexeme>) -> Result<node::Program, String> {
    tokens.reverse(); // TODO: is reversing first faster than pop_back()?
    let mut parser = Parser {
        tokens,
        symbol_table: HashMap::new()
    };
    let mut program: node::Program = Default::default();

    let mut line_numb = 1;
    let mut line = parser.parse_line();
    while !line.as_ref().ok_or(format!("invalid syntax at line {}", line_numb))?.expressions.is_empty() {
        program.statements.push(line.unwrap());

        line = parser.parse_line();
        line_numb += 1;
    }

    Ok(program)
}
