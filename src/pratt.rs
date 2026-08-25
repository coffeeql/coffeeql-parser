//! Pratt Parser — Expression parsing with precedence climbing
//! Used inside .where() conditions

use crate::lexer::Token;
use super::ast::{Expression, BinaryOp};
use super::error::ParseError;

/// Operator precedence levels
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Precedence {
    None       = 0,
    Or         = 1,  // |
    And        = 2,  // , inside where
    Comparison = 3,  // > < >= <= = !=
    Unary      = 4,  // !
    Call       = 5,  // .near() .like()
}

pub struct PrattParser<'a> {
    tokens:  &'a [Token],
    current: usize,
}

impl<'a> PrattParser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn position(&self) -> usize {
        self.current
    }

    /// Parse a full expression
    pub fn parse_expression(
        &mut self,
        min_prec: u8,
    ) -> Result<Expression, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            let prec = self.current_precedence();
            if prec <= min_prec { break; }

            left = self.parse_infix(left, prec)?;
        }

        Ok(left)
    }

    // ── Prefix 

    fn parse_prefix(&mut self) -> Result<Expression, ParseError> {
        match self.current().clone() {

            Token::Int(n)   => { self.advance(); Ok(Expression::Int(n)) }
            Token::Float(f) => { self.advance(); Ok(Expression::Float(f)) }
            Token::Bool(b)  => { self.advance(); Ok(Expression::Bool(b)) }
            Token::Null     => { self.advance(); Ok(Expression::Null) }

            Token::Text(s)  => {
                self.advance();
                Ok(Expression::Text(s))
            }

            Token::Wildcard => {
                self.advance();
                Ok(Expression::Wildcard)
            }

            // Negation: !active
            Token::Bang => {
                self.advance();
                let expr = self.parse_expression(
                    Precedence::Unary as u8
                )?;
                Ok(Expression::Not(Box::new(expr)))
            }

            // Grouped: (age > 18)
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression(0)?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }

            // Collection token in expression — orders[].status, users[].id
            // Happens in .where() after .mix()
            Token::Collection { name, .. } => {
                self.advance();
                if self.current() == &Token::Dot {
                    self.advance();
                    if let Token::Identifier(field) = self.current().clone() {
                        self.advance();
                        return Ok(Expression::NestedField(vec![name, field]));
                    }
                }
                Ok(Expression::Field(name))
            }

            // Identifier or special method
            Token::Identifier(name) => {
                self.advance();
                self.parse_identifier_expr(name)
            }

            // Built-in functions
            Token::FnUuid  => { self.advance(); self.parse_fn_call("uuid") }
            Token::FnNow   => { self.advance(); self.parse_fn_call("now") }
            Token::FnToday => { self.advance(); self.parse_fn_call("today") }
            Token::FnCount => { self.advance(); self.parse_fn_call("COUNT") }
            Token::FnSum   => { self.advance(); self.parse_fn_call("SUM") }
            Token::FnAvg   => { self.advance(); self.parse_fn_call("AVG") }
            Token::FnMax   => { self.advance(); self.parse_fn_call("MAX") }
            Token::FnMin   => { self.advance(); self.parse_fn_call("MIN") }

            other => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found:    format!("{:?}", other),
            })
        }
    }

    // ── Infix

    fn parse_infix(
        &mut self,
        left: Expression,
        prec: u8,
    ) -> Result<Expression, ParseError> {
        match self.current().clone() {

            // Binary operators
            Token::Eq | Token::NotEq |
            Token::Gt | Token::Lt    |
            Token::Gte | Token::Lte  => {
                let op = self.parse_binary_op();
                let right = self.parse_expression(prec)?;
                Ok(Expression::Binary {
                    left:  Box::new(left),
                    op,
                    right: Box::new(right),
                })
            }

            // AND — comma inside .where()
            Token::Comma => {
                self.advance();
                let right = self.parse_expression(
                    Precedence::And as u8
                )?;
                Ok(match left {
                    Expression::And(mut exprs) => {
                        exprs.push(right);
                        Expression::And(exprs)
                    }
                    _ => Expression::And(vec![left, right])
                })
            }

            // OR — pipe
            Token::Pipe => {
                self.advance();
                let right = self.parse_expression(
                    Precedence::Or as u8
                )?;
                Ok(match left {
                    Expression::Or(mut exprs) => {
                        exprs.push(right);
                        Expression::Or(exprs)
                    }
                    _ => Expression::Or(vec![left, right])
                })
            }

            // EXISTS: field EXISTS
            Token::Exists => {
                self.advance();
                if let Expression::Field(name) = left {
                    Ok(Expression::Exists { field: name })
                } else if let Expression::NestedField(parts) = left {
                    Ok(Expression::Exists { field: parts.join(".") })
                } else {
                    Ok(Expression::Exists { field: "unknown".to_string() })
                }
            }

            _ => Ok(left)
        }
    }

    // ── Identifier / Method 

    fn parse_identifier_expr(
        &mut self,
        name: String,
    ) -> Result<Expression, ParseError> {

        // No dot — simple field
        if self.current() != &Token::Dot {
            return Ok(Expression::Field(name));
        }

        self.advance(); // consume dot

        match self.current().clone() {

            // location.near(lat, lon, dist)
            Token::MethodNear => {
                self.advance();
                self.expect(Token::LParen)?;
                let lat  = self.expect_float()?;
                self.expect(Token::Comma)?;
                let lon  = self.expect_float()?;
                self.expect(Token::Comma)?;
                let dist = self.expect_distance()?;
                self.expect(Token::RParen)?;
                Ok(Expression::Near { field: name, lat, lon, distance: dist })
            }

            // embed.like("query").threshold(0.85)
            Token::MethodLike => {
                self.advance();
                self.expect(Token::LParen)?;
                let query = self.expect_string()?;
                self.expect(Token::RParen)?;

                // Optional .threshold()
                let threshold = if self.current() == &Token::Dot {
                    self.advance(); // .
                    if self.current() == &Token::MethodThreshold {
                        self.advance();
                        self.expect(Token::LParen)?;
                        let t = self.expect_float()?;
                        self.expect(Token::RParen)?;
                        t
                    } else { 0.80 }
                } else { 0.80 };

                Ok(Expression::Like { field: name, query, threshold })
            }

            // items.has("latte")
            Token::MethodHas => {
                self.advance();
                self.expect(Token::LParen)?;
                let value = self.parse_expression(0)?;
                self.expect(Token::RParen)?;
                Ok(Expression::Has {
                    field: name,
                    value: Box::new(value),
                })
            }

            // time.last(7d)
            Token::MethodLast => {
                self.advance();
                self.expect(Token::LParen)?;
                let dur = self.expect_duration()?;
                self.expect(Token::RParen)?;
                Ok(Expression::Last { field: name, duration: dur })
            }

            // FIX: Nested field — specs.color, specs.watts
            // This is the key fix for "Expected Comma but found Dot"
            // The nested field becomes a NestedField expression
            // which the infix parser handles correctly with comma separation
            Token::Identifier(field) => {
                self.advance();
                // Return NestedField — infix parser will handle
                // the next operator (>, <, =, etc.) correctly
                eprintln!("DEBUG NestedField: {}.{}, next={:?}", name, field, self.current());
                Ok(Expression::NestedField(vec![name, field]))
            }

            _ => Ok(Expression::Field(name))
        }
    }

    fn parse_fn_call(
        &mut self,
        name: &str,
    ) -> Result<Expression, ParseError> {
        self.expect(Token::LParen)?;
        let mut args = vec![];
        while self.current() != &Token::RParen && !self.is_at_end() {
            args.push(self.parse_expression(0)?);
            if self.current() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;
        Ok(Expression::FnCall {
            name: name.to_string(),
            args,
        })
    }

    // ── Helpers 

    fn current_precedence(&self) -> u8 {
        match self.current() {
            Token::Pipe  => Precedence::Or         as u8,
            Token::Comma => Precedence::And        as u8,
            Token::Eq | Token::NotEq |
            Token::Gt | Token::Lt |
            Token::Gte | Token::Lte
                         => Precedence::Comparison as u8,
            Token::Exists => Precedence::Comparison as u8,
            _            => Precedence::None       as u8,
        }
    }

    fn parse_binary_op(&mut self) -> BinaryOp {
        let op = match self.current() {
            Token::Eq    => BinaryOp::Eq,
            Token::NotEq => BinaryOp::NotEq,
            Token::Gt    => BinaryOp::Gt,
            Token::Lt    => BinaryOp::Lt,
            Token::Gte   => BinaryOp::Gte,
            Token::Lte   => BinaryOp::Lte,
            _            => BinaryOp::Eq,
        };
        self.advance();
        op
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current(), Token::Eof)
    }

    fn expect(&mut self, tok: Token) -> Result<(), ParseError> {
        if self.current() == &tok {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", tok),
                found:    format!("{:?}", self.current()),
            })
        }
    }

    fn expect_float(&mut self) -> Result<f64, ParseError> {
        match self.current().clone() {
            Token::Float(f) => { self.advance(); Ok(f) }
            Token::Int(n)   => { self.advance(); Ok(n as f64) }
            other => Err(ParseError::UnexpectedToken {
                expected: "number".to_string(),
                found:    format!("{:?}", other),
            })
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.current().clone() {
            Token::Text(s) => { self.advance(); Ok(s) }
            other => Err(ParseError::UnexpectedToken {
                expected: "string".to_string(),
                found:    format!("{:?}", other),
            })
        }
    }

    fn expect_distance(
        &mut self,
    ) -> Result<crate::lexer::token::Distance, ParseError> {
        match self.current().clone() {
            Token::Distance(d) => { self.advance(); Ok(d) }
            other => Err(ParseError::UnexpectedToken {
                expected: "distance (e.g. 5km)".to_string(),
                found:    format!("{:?}", other),
            })
        }
    }

    fn expect_duration(
        &mut self,
    ) -> Result<crate::lexer::token::Duration, ParseError> {
        match self.current().clone() {
            Token::Duration(d) => { self.advance(); Ok(d) }
            other => Err(ParseError::UnexpectedToken {
                expected: "duration (e.g. 7d)".to_string(),
                found:    format!("{:?}", other),
            })
        }
    }
}
