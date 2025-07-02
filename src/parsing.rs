use derive_more::Display;
use thiserror::Error;

use crate::lexer::{Lexer, LexerError, LexerErrorKind, Location, Token, TokenKind};

#[derive(Debug, Error)]
#[error("{location}: {kind}")]
pub struct ParseError<'source> {
    pub location: Location,
    pub kind: ParseErrorKind<'source>,
}

impl From<LexerError> for ParseError<'_> {
    fn from(error: LexerError) -> Self {
        ParseError {
            location: error.location,
            kind: ParseErrorKind::LexerError(error.kind),
        }
    }
}

#[derive(Debug, Display)]
pub enum ParseErrorKind<'source> {
    #[display("{_0}")]
    LexerError(LexerErrorKind),
    #[display("Unexpected end of input")]
    UnexpectedEOI,
    #[display("Unexpected token '{_0}'")]
    UnexpectedToken(Token<'source>),
}

#[derive(Debug)]
pub struct AstStatement<'source> {
    pub location: Location,
    pub kind: AstStatementKind<'source>,
}

#[derive(Debug)]
pub enum AstStatementKind<'source> {
    Assignment {
        name: &'source str,
        name_token: Token<'source>,
        equals_token: Token<'source>,
        value: AstExpression<'source>,
    },
}

#[derive(Debug)]
pub struct AstExpression<'source> {
    pub location: Location,
    pub kind: AstExpressionKind<'source>,
}

#[derive(Debug)]
pub enum AstExpressionKind<'source> {
    Name {
        name: &'source str,
        name_token: Token<'source>,
    },
    Number {
        number: f32,
        number_token: Token<'source>,
    },
    Unary {
        operator: UnaryOperator,
        operator_token: Token<'source>,
        operand: Box<AstExpression<'source>>,
    },
    Binary {
        left: Box<AstExpression<'source>>,
        operator: BinaryOperator,
        operator_token: Token<'source>,
        right: Box<AstExpression<'source>>,
    },
}

#[derive(Debug)]
pub enum UnaryOperator {
    Negate,
    Dual,
    Reverse,
}

#[derive(Debug)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Wedge,
    Inner,
    Regressive,
}

pub fn parse(source: &str) -> Result<Vec<AstStatement<'_>>, ParseError<'_>> {
    let mut parser = Parser::new(source);

    let mut statements = vec![];
    while parser.lexer.peek_token()?.is_some() {
        statements.push(parser.parse_statement()?);
    }
    Ok(statements)
}

struct Parser<'source> {
    lexer: Lexer<'source>,
}

macro_rules! expect_token {
    ($parser:ident, $pattern:pat $(, $pattern_names:ident)*) => {
        match $parser.lexer.next_token() {
            Ok(Some(token @ Token {
                location: _,
                kind: $pattern,
            })) => Ok((token $(, $pattern_names)*)),
            #[allow(unreachable_patterns)]
            Ok(Some(token)) => Err(ParseError {
                location: token.location,
                kind: ParseErrorKind::UnexpectedToken(token),
            }),
            Ok(None) => Err(ParseError {
                location: $parser.lexer.location(),
                kind: ParseErrorKind::UnexpectedEOI,
            }),
            Err(error) => Err(error.into()),
        }
    };
}

impl<'source> Parser<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            lexer: Lexer::new(source),
        }
    }

    fn parse_statement(&mut self) -> Result<AstStatement<'source>, ParseError<'source>> {
        let (name_token, name) = expect_token!(self, TokenKind::Name(name), name)?;
        let equals_token = expect_token!(self, TokenKind::Equal)?;
        let value = self.parse_expression()?;
        expect_token!(self, TokenKind::Semicolon)?;
        Ok(AstStatement {
            location: equals_token.location,
            kind: AstStatementKind::Assignment {
                name,
                name_token,
                equals_token,
                value,
            },
        })
    }

    fn parse_expression(&mut self) -> Result<AstExpression<'source>, ParseError<'source>> {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(
        &mut self,
        parent_precedence: usize,
    ) -> Result<AstExpression<'source>, ParseError<'source>> {
        let unary_operator = match self.lexer.peek_token()?.map(|token| token.kind) {
            Some(TokenKind::Minus) => Some(UnaryOperator::Negate),
            Some(TokenKind::ExclamationMark) => Some(UnaryOperator::Dual),
            Some(TokenKind::Tilde) => Some(UnaryOperator::Reverse),
            _ => None,
        };
        let mut left = if let Some(operator) = unary_operator {
            let operator_token = expect_token!(self, _)?;
            let operand = self.parse_binary_expression(usize::MAX)?;
            AstExpression {
                location: operator_token.location,
                kind: AstExpressionKind::Unary {
                    operator,
                    operator_token,
                    operand: Box::new(operand),
                },
            }
        } else {
            self.parse_primary_expression()?
        };

        loop {
            let (precedence, operator) = match self.lexer.peek_token()?.map(|token| token.kind) {
                Some(TokenKind::Plus) => (1, BinaryOperator::Add),
                Some(TokenKind::Minus) => (1, BinaryOperator::Subtract),
                Some(TokenKind::Asterisk) => (2, BinaryOperator::Multiply),
                Some(TokenKind::Slash) => (2, BinaryOperator::Divide),
                Some(TokenKind::Caret) => (2, BinaryOperator::Wedge),
                Some(TokenKind::Pipe) => (2, BinaryOperator::Inner),
                Some(TokenKind::Ampersand) => (2, BinaryOperator::Regressive),
                _ => break,
            };

            if precedence <= parent_precedence {
                break;
            }
            let operator_token = expect_token!(self, _)?;

            let right = self.parse_binary_expression(precedence)?;
            left = AstExpression {
                location: operator_token.location,
                kind: AstExpressionKind::Binary {
                    left: Box::new(left),
                    operator,
                    operator_token,
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_primary_expression(&mut self) -> Result<AstExpression<'source>, ParseError<'source>> {
        Ok(match expect_token!(self, _)? {
            name_token @ Token {
                location,
                kind: TokenKind::Name(name),
            } => AstExpression {
                location,
                kind: AstExpressionKind::Name { name, name_token },
            },

            number_token @ Token {
                location,
                kind: TokenKind::Number(number),
            } => AstExpression {
                location,
                kind: AstExpressionKind::Number {
                    number,
                    number_token,
                },
            },

            Token {
                location: _,
                kind: TokenKind::OpenParenthesis,
            } => {
                let expression = self.parse_expression()?;
                expect_token!(self, TokenKind::CloseParenthesis)?;
                expression
            }

            token => {
                return Err(ParseError {
                    location: token.location,
                    kind: ParseErrorKind::UnexpectedToken(token),
                });
            }
        })
    }
}
