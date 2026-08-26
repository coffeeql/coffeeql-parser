//! Recursive Descent Parser
//! Handles: query structure, chain operations, shot, grind, menu

use coffeeql_lexer::token::Token;
use coffeeql_lexer::token::{CollectionKind, SortDir, DataType, Constraint};
use crate::ast::*;
use super::error::ParseError;
use super::pratt::{PrattParser, Precedence};

/// Chain state machine — enforces valid ordering
#[derive(Debug, Clone, PartialEq)]
enum ChainState {
    Start,
    AfterWhere,
    AfterMix,
    AfterBlend,
    AfterGive,
    AfterSort,
    AfterCup,
    AfterPour,
    AfterRefill,
    Done,
}

pub struct RecursiveParser {
    tokens: Vec<Token>,
    current: usize,
}

impl RecursiveParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    // ── Entry

    pub fn parse(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = vec![];
        while !self.is_at_end() {
            let stmt = match self.current().clone() {
                Token::Shot => self.parse_shot()?,
                Token::Grind => self.parse_grind()?,
                Token::Menu => self.parse_menu()?,
                Token::Collection { .. } => self.parse_query_stmt()?,
                other => return Err(ParseError::UnexpectedToken {
                    expected: "query, shot, grind, or menu".to_string(),
                    found: format!("{:?}", other),
                }),
            };
            statements.push(stmt);
        }
        Ok(statements)
    }

    // ── Query

    fn parse_query_stmt(&mut self) -> Result<Statement, ParseError> {
        Ok(Statement::Query(self.parse_query()?))
    }

    fn parse_query(&mut self) -> Result<QueryNode, ParseError> {
        let (name, kind) = self.expect_collection()?;
        let mut chain = vec![];
        let mut state = ChainState::Start;

        while self.current() == &Token::Dot {
            self.advance(); // consume dot
            let op = self.parse_chain_op(&mut state)?;
            chain.push(op);
            if state == ChainState::AfterCup
                || state == ChainState::Done {
                break;
            }
        }

        Ok(QueryNode { collection: name, kind, chain })
    }

    // ── Chain

    fn parse_chain_op(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        match self.current().clone() {
            Token::Where  => self.parse_where(state),
            Token::Give   => self.parse_give(state),
            Token::Sort   => self.parse_sort(state),
            Token::Cup    => self.parse_cup(state),
            Token::Blend  => self.parse_blend(state),
            Token::Mix    => self.parse_mix(state),
            Token::Pour   => self.parse_pour(state),
            Token::Refill => self.parse_refill(state),
            Token::Spill  => {
                self.advance();
                self.expect(Token::LParen)?;
                self.expect(Token::RParen)?;
                *state = ChainState::Done;
                Ok(ChainOp::Spill)
            }
            other => Err(ParseError::UnknownChainMethod {
                method: format!("{:?}", other),
            })
        }
    }

    // ── Where

    fn parse_where(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        self.validate_state(state, "where", &[
            ChainState::Start,
            ChainState::AfterMix,
        ])?;
        self.advance(); // consume `where`
        self.expect(Token::LParen)?;
        let condition = self.parse_expr_until(Token::RParen)?;
        self.expect(Token::RParen)?;
        *state = ChainState::AfterWhere;
        Ok(ChainOp::Where(WhereNode { condition }))
    }

    // ── Give

    fn parse_give(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        self.validate_state(state, "give", &[
            ChainState::Start,
            ChainState::AfterWhere,
            ChainState::AfterBlend,
            ChainState::AfterMix,
        ])?;
        self.advance(); // consume `give`
        self.expect(Token::LParen)?;
        let mut fields = vec![];
        loop {
            fields.push(self.parse_field_expr()?);
            if self.current() != &Token::Comma { break; }
            self.advance(); // consume comma
        }
        self.expect(Token::RParen)?;
        *state = ChainState::AfterGive;
        Ok(ChainOp::Give(GiveNode { fields }))
    }

    // ── Sort

    fn parse_sort(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        self.validate_state(state, "sort", &[
            ChainState::AfterWhere,
            ChainState::AfterGive,
        ])?;
        self.advance(); // consume `sort`
        self.expect(Token::LParen)?;
        let field = self.expect_dot_field()?;
        self.expect(Token::Comma)?;
        let direction = match self.current() {
            Token::Asc  => { self.advance(); SortDir::Asc }
            Token::Desc => { self.advance(); SortDir::Desc }
            other => return Err(ParseError::InvalidSortDir {
                got: format!("{:?}", other),
            })
        };
        self.expect(Token::RParen)?;
        *state = ChainState::AfterSort;
        Ok(ChainOp::Sort(SortNode { field, direction }))
    }

    // ── Cup

    fn parse_cup(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        self.validate_state(state, "cup", &[
            ChainState::Start,
            ChainState::AfterWhere,
            ChainState::AfterGive,
            ChainState::AfterSort,
            ChainState::AfterBlend,
        ])?;
        self.advance(); // consume `cup`
        self.expect(Token::LParen)?;
        let limit = match self.current().clone() {
            Token::Int(n) if n > 0 => { self.advance(); n as u64 }
            other => return Err(ParseError::InvalidCupLimit {
                got: format!("{:?}", other),
            })
        };
        self.expect(Token::RParen)?;
        *state = ChainState::AfterCup;
        Ok(ChainOp::Cup(CupNode { limit }))
    }

    // ── Blend

    fn parse_blend(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        self.validate_state(state, "blend", &[
            ChainState::AfterWhere,
            ChainState::Start,
        ])?;
        self.advance(); // consume `blend`
        self.expect(Token::LParen)?;
        let field = self.expect_identifier()?;
        self.expect(Token::RParen)?;
        *state = ChainState::AfterBlend;
        Ok(ChainOp::Blend(BlendNode { field }))
    }

    // ── Mix

    fn parse_mix(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        self.validate_state(state, "mix", &[
            ChainState::Start,
            ChainState::AfterWhere,
        ])?;
        self.advance(); // consume `mix`
        self.expect(Token::LParen)?;
        let (coll_name, coll_kind) = self.expect_collection()?;

        match self.current().clone() {
            Token::On => { self.advance(); }
            Token::Identifier(s) if s.to_uppercase() == "ON" => { self.advance(); }
            other => return Err(ParseError::UnexpectedToken {
                expected: "ON".to_string(),
                found: format!("{:?}", other),
            })
        }

        let left_field  = self.expect_dot_field()?;
        self.expect(Token::Eq)?;
        let right_field = self.expect_dot_field()?;
        self.expect(Token::RParen)?;
        *state = ChainState::AfterMix;
        Ok(ChainOp::Mix(MixNode {
            collection: coll_name,
            kind: coll_kind,
            left_field,
            right_field,
        }))
    }

    // ── Pour

    fn parse_pour(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        self.advance(); // consume `pour`
        self.expect(Token::LParen)?;
        let data = self.parse_object()?;
        self.expect(Token::RParen)?;
        *state = ChainState::AfterPour;
        Ok(ChainOp::Pour(PourNode { data }))
    }

    // ── Refill

    fn parse_refill(
        &mut self,
        state: &mut ChainState,
    ) -> Result<ChainOp, ParseError> {
        self.validate_state(state, "refill", &[
            ChainState::AfterWhere,
        ])?;
        self.advance(); // consume `refill`
        self.expect(Token::LParen)?;
        let data = self.parse_object()?;
        self.expect(Token::RParen)?;
        *state = ChainState::AfterRefill;
        Ok(ChainOp::Refill(RefillNode { data }))
    }

    // ── Field Expression

    fn parse_field_expr(&mut self) -> Result<FieldExpr, ParseError> {
        if self.current() == &Token::Wildcard {
            self.advance();
            return Ok(FieldExpr::Wildcard);
        }

        let agg = match self.current() {
            Token::FnCount => Some(AggFunc::Count),
            Token::FnSum   => Some(AggFunc::Sum),
            Token::FnAvg   => Some(AggFunc::Avg),
            Token::FnMax   => Some(AggFunc::Max),
            Token::FnMin   => Some(AggFunc::Min),
            _ => None,
        };

        if let Some(func) = agg {
            self.advance();
            self.expect(Token::LParen)?;
            let field = if self.current() != &Token::RParen {
                Some(self.expect_identifier()?)
            } else { None };
            self.expect(Token::RParen)?;
            self.expect(Token::As)?;
            let alias = self.expect_identifier()?;
            return Ok(FieldExpr::Aggregate { func, field, alias });
        }

        let name = match self.current().clone() {
            Token::Identifier(s) => { self.advance(); s }
            Token::Collection { name, .. } => { self.advance(); name }
            other => return Err(ParseError::UnexpectedToken {
                expected: "field name".to_string(),
                found: format!("{:?}", other),
            })
        };

        if self.current() == &Token::Dot {
            self.advance();
            let sub = self.expect_identifier()?;
            return Ok(FieldExpr::Nested(vec![name, sub]));
        }

        Ok(FieldExpr::Simple(name))
    }

    // ── Object Expression

    fn parse_object(&mut self) -> Result<ObjectExpr, ParseError> {
        self.expect(Token::LBrace)?;
        let mut fields = vec![];
        while self.current() != &Token::RBrace && !self.is_at_end() {
            let key = match self.current().clone() {
                Token::Identifier(s) => { self.advance(); s }
                Token::Text(s)       => { self.advance(); s }
                other => return Err(ParseError::UnexpectedToken {
                    expected: "field key (identifier or string)".to_string(),
                    found: format!("{:?}", other),
                })
            };
            self.expect(Token::Colon)?;
            let val = self.parse_object_value()?;
            fields.push((key, val));
            if self.current() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(ObjectExpr { fields })
    }

    fn parse_object_value(&mut self) -> Result<Expression, ParseError> {
        match self.current().clone() {
            Token::LBrace => {
                let obj = self.parse_object()?;
                Ok(Expression::FnCall {
                    name: "__object__".to_string(),
                    args: obj.fields.into_iter().map(|(_, v)| v).collect(),
                })
            }
            Token::LBracket => {
                self.advance();
                let mut items = vec![];
                while self.current() != &Token::RBracket && !self.is_at_end() {
                    items.push(self.parse_object_value()?);
                    if self.current() == &Token::Comma { self.advance(); }
                }
                if self.current() == &Token::RBracket { self.advance(); }
                Ok(Expression::FnCall {
                    name: "__array__".to_string(),
                    args: items,
                })
            }
            Token::Text(s)  => { self.advance(); Ok(Expression::Text(s)) }
            Token::Int(n)   => { self.advance(); Ok(Expression::Int(n)) }
            Token::Float(f) => { self.advance(); Ok(Expression::Float(f)) }
            Token::Bool(b)  => { self.advance(); Ok(Expression::Bool(b)) }
            Token::Null     => { self.advance(); Ok(Expression::Null) }
            Token::FnUuid => {
                self.advance();
                self.expect(Token::LParen)?;
                self.expect(Token::RParen)?;
                Ok(Expression::FnCall { name: "uuid".to_string(), args: vec![] })
            }
            Token::FnNow => {
                self.advance();
                self.expect(Token::LParen)?;
                self.expect(Token::RParen)?;
                Ok(Expression::FnCall { name: "now".to_string(), args: vec![] })
            }
            Token::FnToday => {
                self.advance();
                self.expect(Token::LParen)?;
                self.expect(Token::RParen)?;
                Ok(Expression::FnCall { name: "today".to_string(), args: vec![] })
            }
            _ => {
                let remaining = &self.tokens[self.current..];
                let mut pratt = PrattParser::new(remaining);
                let expr = pratt.parse_expression(Precedence::And as u8)?;
                self.current += pratt.position();
                Ok(expr)
            }
        }
    }

    // ── Shot

    fn parse_shot(&mut self) -> Result<Statement, ParseError> {
        self.advance(); // consume `shot`
        self.expect(Token::LBrace)?;
        let mut queries = vec![];
        while self.current() != &Token::RBrace {
            if self.is_at_end() {
                return Err(ParseError::UnexpectedEof {
                    hint: "Close shot block with '}'".to_string(),
                });
            }
            queries.push(self.parse_query()?);
        }
        if queries.is_empty() {
            return Err(ParseError::EmptyShot);
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::Shot(ShotNode { queries }))
    }

    // ── Grind

    fn parse_grind(&mut self) -> Result<Statement, ParseError> {
        self.advance(); // consume `grind`
        let (name, kind) = self.expect_collection()?;
        let (schema, flex) = if self.current() == &Token::LParen {
            self.advance();
            let mut fields = vec![];
            while self.current() != &Token::RParen && !self.is_at_end() {
                let field_name = self.expect_identifier()?;
                let data_type  = self.expect_data_type()?;
                let mut constraints = vec![];
                while let Some(c) = self.try_constraint() {
                    constraints.push(c);
                }
                fields.push(SchemaField { name: field_name, data_type, constraints });
                if self.current() == &Token::Comma { self.advance(); }
            }
            self.expect(Token::RParen)?;
            let flex = if self.current() == &Token::Flex {
                self.advance(); true
            } else { false };
            (Some(fields), flex)
        } else {
            (None, false)
        };
        Ok(Statement::Grind(GrindNode { collection: name, kind, schema, flex }))
    }

    // ── Menu

    fn parse_menu(&mut self) -> Result<Statement, ParseError> {
        self.advance(); // consume `menu`
        self.expect(Token::LParen)?;
        let collection = if self.current() != &Token::RParen {
            Some(self.expect_collection()?)
        } else { None };
        self.expect(Token::RParen)?;
        Ok(Statement::Menu(MenuNode { collection }))
    }

    // ── Helpers

    fn parse_expr_until(
        &mut self,
        _stop: Token,
    ) -> Result<Expression, ParseError> {
        let remaining = &self.tokens[self.current..];
        let mut pratt = PrattParser::new(remaining);
        let expr = pratt.parse_expression(0)?;
        self.current += pratt.position();
        Ok(expr)
    }

    fn expect_collection(
        &mut self,
    ) -> Result<(String, CollectionKind), ParseError> {
        match self.current().clone() {
            Token::Collection { name, kind } => {
                self.advance();
                Ok((name, kind))
            }
            other => Err(ParseError::UnexpectedToken {
                expected: "collection (e.g. users[] or products{})".to_string(),
                found: format!("{:?}", other),
            })
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.current().clone() {
            Token::Identifier(s) => { self.advance(); Ok(s) }
            other => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: format!("{:?}", other),
            })
        }
    }

    fn expect_dot_field(&mut self) -> Result<String, ParseError> {
        let base = match self.current().clone() {
            Token::Identifier(s)     => { self.advance(); s }
            Token::Collection { name, .. } => { self.advance(); name }
            other => return Err(ParseError::UnexpectedToken {
                expected: "identifier or collection".to_string(),
                found: format!("{:?}", other),
            })
        };
        if self.current() == &Token::Dot {
            self.advance();
            let field = self.expect_identifier()?;
            return Ok(format!("{}.{}", base, field));
        }
        Ok(base)
    }

    fn expect_data_type(&mut self) -> Result<DataType, ParseError> {
        match self.current().clone() {
            Token::DataType(dt) => { self.advance(); Ok(dt) }
            other => Err(ParseError::UnexpectedToken {
                expected: "data type (UUID, TEXT, INT, FLOAT, BOOL, DATETIME, GEOPOINT, VECTOR)".to_string(),
                found: format!("{:?}", other),
            })
        }
    }

    fn try_constraint(&mut self) -> Option<Constraint> {
        match self.current().clone() {
            Token::Constraint(c) => { self.advance(); Some(c) }
            _ => None,
        }
    }

    fn validate_state(
        &self,
        state: &ChainState,
        method: &str,
        valid: &[ChainState],
    ) -> Result<(), ParseError> {
        if valid.contains(state) { return Ok(()); }
        Err(ParseError::WrongChainOrder {
            found: method.to_string(),
            after: format!("{:?}", state),
            hint:  format!(".{}() is not valid after {:?}", method, state),
        })
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

    fn current(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.current() == &Token::Eof
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coffeeql_lexer::token::{Token, CollectionKind};

    /// Build a token stream manually — no Lexer dependency needed.
    /// The lexer is a separate public crate; tests here work at the
    /// parser level and construct tokens directly.
    fn make_tokens(tokens: Vec<Token>) -> Vec<Token> {
        let mut t = tokens;
        t.push(Token::Eof);
        t
    }

    #[test]
    fn parse_simple_query() {
        // users[].where(age > 18).give(name).cup(10)
        let tokens = make_tokens(vec![
            Token::Collection { name: "users".into(), kind: CollectionKind::Structured },
            Token::Dot,
            Token::Where,
            Token::LParen,
            Token::Identifier("age".into()),
            Token::Gt,
            Token::Int(18),
            Token::RParen,
            Token::Dot,
            Token::Give,
            Token::LParen,
            Token::Identifier("name".into()),
            Token::RParen,
            Token::Dot,
            Token::Cup,
            Token::LParen,
            Token::Int(10),
            Token::RParen,
        ]);
        let stmts = RecursiveParser::new(tokens).parse().unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::Query(_)));
    }

    #[test]
    fn parse_pour_object() {
        // users[].pour({ name: "Rahul", age: 25 })
        let tokens = make_tokens(vec![
            Token::Collection { name: "users".into(), kind: CollectionKind::Structured },
            Token::Dot,
            Token::Pour,
            Token::LParen,
            Token::LBrace,
            Token::Identifier("name".into()),
            Token::Colon,
            Token::Text("Rahul".into()),
            Token::Comma,
            Token::Identifier("age".into()),
            Token::Colon,
            Token::Int(25),
            Token::RBrace,
            Token::RParen,
        ]);
        let stmts = RecursiveParser::new(tokens).parse().unwrap();
        assert!(matches!(&stmts[0], Statement::Query(_)));
    }

    #[test]
    fn parse_menu() {
        // menu()
        let tokens = make_tokens(vec![
            Token::Menu,
            Token::LParen,
            Token::RParen,
        ]);
        let stmts = RecursiveParser::new(tokens).parse().unwrap();
        assert!(matches!(&stmts[0], Statement::Menu(_)));
    }

    #[test]
    fn parse_wrong_chain_order_errors() {
        // sort is NOT valid from Start — only valid AfterWhere or AfterGive
        let tokens = make_tokens(vec![
            Token::Collection { name: "users".into(), kind: CollectionKind::Structured },
            Token::Dot,
            Token::Sort,
            Token::LParen,
            Token::Identifier("name".into()),
            Token::Comma,
            Token::Asc,
            Token::RParen,
        ]);
        let result = RecursiveParser::new(tokens).parse();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::WrongChainOrder { .. }));
    }
}
