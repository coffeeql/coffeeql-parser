//! CoffeeQL Parser
//! Recursive Descent (structure) + Pratt (expressions)

pub mod ast;
pub mod recursive;
pub mod pratt;
pub mod error;

use crate::lexer::Token;
use ast::Statement;
pub use error::ParseError;
use recursive::RecursiveParser;

pub struct Parser {
    tokens: Vec<Token>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens }
    }

    pub fn parse(&self) -> Result<Vec<Statement>, ParseError> {
        RecursiveParser::new(self.tokens.clone()).parse()
    }
}
