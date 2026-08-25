//! CoffeeQL Parser Errors

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ParseError {
    #[error("☕ Wrong order!\n\
             '.{found}()' cannot come after '.{after}()'\n\
             Hint: {hint}")]
    WrongChainOrder {
        found: String,
        after: String,
        hint:  String,
    },

    #[error("☕ Expected '{expected}' but found '{found}'")]
    UnexpectedToken {
        expected: String,
        found:    String,
    },

    #[error("☕ Cup is empty!\n\
             Query ended unexpectedly.\n\
             Hint: {hint}")]
    UnexpectedEof {
        hint: String,
    },

    #[error("☕ Unknown chain method '.{method}()'\n\
             Valid: where, give, sort, cup, blend, mix, pour, refill, spill")]
    UnknownChainMethod {
        method: String,
    },

    #[error("☕ .cup() must have a positive number!\n\
             Got: {got}")]
    InvalidCupLimit {
        got: String,
    },

    #[error("☕ .sort() direction must be ASC or DESC\n\
             Got: '{got}'")]
    InvalidSortDir {
        got: String,
    },

    #[error("☕ shot{{}} block must have at least one query")]
    EmptyShot,
}
