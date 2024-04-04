// recursive descent parser
use lexer::*;

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
        Variable()
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
    tokens: &'a mut Vec<Lexeme>
}

impl Parser<'_> {
    // used to evaluate integer literal expressions involving increment and decrement
    fn eval_lit(&mut self) -> isize {
        let mut value = Default::default();

        while let Some(lexeme) = self.tokens.pop() {
            if lexeme == Lexeme::Token(Token::LineBreak) {
                break;
            }

            value += (lexeme == Lexeme::Token(Token::Increment)) as isize;
            value -= (lexeme == Lexeme::Token(Token::Decrement)) as isize;
        }

        value
    }


    fn parse_int(&mut self) -> Option<isize> {
        Some(self.eval_lit())
    }

    // TODO: return type Result with ScopeError type that can be error for condition, body, etc.
    fn parse_scope(&mut self) -> Option<node::Scope> {
        let mut scope: node::Scope = Default::default();

        // TODO: should we validate that its a ScopeStart since we do it in parse_statement() already
        scope.kind = match self.tokens.pop() {
            Some(Lexeme::Token(Token::ScopeStart)) => {
                match self.tokens.first() {
                    Some(Lexeme::Token(Token::Access)) => node::ScopeType::Function,
                    Some(Lexeme::Token(Token::Repeat)) => node::ScopeType::Loop,
                    // TODO: if statement
                    _ => return None
                }
            },
            _ => return None
        };

        scope.signature = self.parse_statement();

        scope.body = parse(&mut self.tokens).unwrap();

        Some(scope)
    }

    fn parse_quote(&mut self) -> Option<node::Expression> {
        let string = node::Expression::StringLit(self.parse_int()?.to_string());
        self.tokens.pop(); // pops the ending quote
        Some(string)
    }

    fn parse_statement(&mut self) -> Option<node::Statement> {
        use node::Expression::*;

        let mut statement: node::Statement = Default::default();

        // TODO: should node::Expressions be put here or should the parsing functions return them?
        // TODO: replace unwraps with proper error handling
        while let Some(lexeme) = self.tokens.pop() {
            statement.expressions.push(match lexeme {
                Lexeme::Token(Token::Zero)      => IntLit(self.parse_int()?),
                Lexeme::Token(Token::Increment) => unreachable!(),
                Lexeme::Token(Token::Decrement) => unreachable!(),
                Lexeme::Token(Token::Access)    => todo!(),
                Lexeme::Token(Token::Repeat)    => unreachable!(),
                Lexeme::Token(Token::Quote)     => self.parse_quote()?,
                Lexeme::Token(Token::Variable)  => unreachable!(),
                Lexeme::Token(Token::ScopeStart)=> Scope(self.parse_scope()?),
                Lexeme::Token(Token::ScopeEnd)  => return Some(statement),

                Lexeme::Identifier(id)          => todo!(),

                Lexeme::Token(Token::LineBreak) => return Some(statement),
            });
        }

        None
    }
}

pub fn parse(tokens: &mut Vec<Lexeme>) -> Result<node::Program, String> {
    tokens.reverse(); // TODO: is reversing first faster than pop_back()?
    let mut parser = Parser {
        tokens
    };
    let mut program: node::Program = Default::default();

    let mut line_numb = 1;
    let mut statement = parser.parse_statement();
    while !statement.as_ref().ok_or(format!("invalid syntax at line {}", line_numb))?.expressions.is_empty() {
        program.statements.push(statement.unwrap());
        // TODO: cancel early if end of scope or dont call parse() in parse_scope()
        // if statement.last() == Some( {
        //
        // }

        statement = parser.parse_statement();
        line_numb += 1;
    }

    Ok(program)
}
