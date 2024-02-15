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

    #[derive(Debug)]
    pub struct Loop {
        pub condition: Expression,   // condition to continue if not zero
        pub body: Program
    }
}

struct Parser {
    tokens: VecDeque<Lexeme>
}

// TODO: not even sure this works
// TODO: when it works move it to lexer aswell to use it there maybe
macro_rules! match_lexeme {
    ($a:expr, $b:pat) => {
        match $a {
            Lexeme::Token(($b, _)) => true,
            _ => false
        }
    }
}

impl Parser {
    // used to evaluate integer literal expressions involving increment and decrement
    fn eval_lit(&mut self) -> isize {
        let mut value = Default::default();

        while let Some(lexeme) = self.tokens.pop_front() {
            // if let Lexeme::Token((Token::LineBreak, _)) = lexeme {
            //     break;
            // }
            if match_lexeme!(lexeme, Token::LineBreak) {
                break;
            }

            value += match_lexeme!(lexeme, Token::Increment) as isize;
            value -= match_lexeme!(lexeme, Token::Decrement) as isize;
        }

        value
    }


    fn parse_expr(&mut self) -> Option<node::Expression> {
        match self.tokens.front() {
            Some(Lexeme::Token((token, _))) => {
                if *token != Token::Zero  {
                    return None
                }
                Some(node::Expression{value: self.eval_lit()})
            }

            // Lexeme::Identifier(id) =>

            _ => None
        }
    }

    // fn parse_loop(&mut self) -> Option<node::Loop> {
    //
    // }
}

pub fn parse(mut tokens: VecDeque<Lexeme>) -> Result<node::Program, String> {
    let mut parser = Parser {
        tokens
    };
    let mut program: node::Program = Default::default();

    while let Some(lexeme) = parser.tokens.pop_front() {
            match lexeme {
                Lexeme::Token((Token::Zero, _))         => {}
                Lexeme::Token((Token::Increment, _))    => {}
                Lexeme::Token((Token::Decrement, _))    => {}
                Lexeme::Token((Token::Access, _))       => {}
                Lexeme::Token((Token::Repeat, _))       => {}
                Lexeme::Token((Token::Quote, _))        => {}
                Lexeme::Token((Token::LineBreak, _))    => {}

                Lexeme::Token((Token::ScopeStart, _))   => {}
                Lexeme::Token((Token::ScopeEnd, _))     => {}

                Lexeme::Identifier((id, _)) => {}

                Lexeme::Token((Token::Variable, _)) => {} // not possible
            }
    }

    Ok(program)
}
