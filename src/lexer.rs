use derive_more::{Debug, Display};
use std::{iter::Peekable, str::CharIndices};
use thiserror::Error;

#[derive(Debug, Display, Clone, Copy)]
#[display("{line}:{column}")]
pub struct Location {
    pub position: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Error)]
#[error("{location}: {kind}")]
pub struct LexerError {
    pub location: Location,
    pub kind: LexerErrorKind,
}

#[derive(Debug, Display)]
pub enum LexerErrorKind {
    #[display("Unexpected character '{_0}'")]
    UnexpectedChar(char),
    #[display("Invalid number")]
    InvalidNumber,
}

#[derive(Debug, Display)]
#[display("{kind}")]
pub struct Token<'source> {
    pub location: Location,
    pub kind: TokenKind<'source>,
}

#[derive(Debug, Display)]
pub enum TokenKind<'source> {
    #[display("{_0}")]
    Name(&'source str),
    #[display("normalize")]
    NormalizeKeyword,
    #[display("magnitude")]
    MagnitudeKeyword,
    #[display("sin")]
    SinKeyword,
    #[display("cos")]
    CosKeyword,
    #[display("asin")]
    ASinKeyword,
    #[display("acos")]
    ACosKeyword,
    #[display("{_0}")]
    Number(f32),
    #[display("(")]
    OpenParenthesis,
    #[display(")")]
    CloseParenthesis,
    #[display(";")]
    Semicolon,
    #[display("+")]
    Plus,
    #[display("-")]
    Minus,
    #[display("*")]
    Asterisk,
    #[display("/")]
    Slash,
    #[display("^")]
    Caret,
    #[display("|")]
    Pipe,
    #[display("&")]
    Ampersand,
    #[display("!")]
    ExclamationMark,
    #[display("~")]
    Tilde,
    #[display("=")]
    Equal,
}

#[derive(Clone)]
pub struct Lexer<'source> {
    source: &'source str,
    chars: Peekable<CharIndices<'source>>,
    location: Location,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            location: Location {
                position: 0,
                line: 1,
                column: 1,
            },
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        let &(_, c) = self.chars.peek()?;
        Some(c)
    }

    fn next_char(&mut self) -> Option<char> {
        let (_, c) = self.chars.next()?;
        self.location.position = self.chars.peek().map_or(self.source.len(), |&(i, _)| i);

        self.location.column += 1;
        if c == '\n' {
            self.location.line += 1;
            self.location.column = 1;
        }

        Some(c)
    }

    pub fn location(&self) -> Location {
        self.location
    }

    pub fn next_token(&mut self) -> Result<Option<Token<'source>>, LexerError> {
        loop {
            let start_location = self.location;
            break Ok(Some(Token {
                location: start_location,
                kind: match self.next_char() {
                    None => return Ok(None),

                    Some('(') => TokenKind::OpenParenthesis,
                    Some(')') => TokenKind::CloseParenthesis,
                    Some(';') => TokenKind::Semicolon,
                    Some('+') => TokenKind::Plus,
                    Some('-') => TokenKind::Minus,
                    Some('*') => TokenKind::Asterisk,
                    Some('/') => TokenKind::Slash,
                    Some('^') => TokenKind::Caret,
                    Some('|') => TokenKind::Pipe,
                    Some('&') => TokenKind::Ampersand,
                    Some('!') => TokenKind::ExclamationMark,
                    Some('~') => TokenKind::Tilde,
                    Some('=') => TokenKind::Equal,

                    Some(c) if c.is_alphabetic() || c == '_' => {
                        while let Some(c) = self.peek_char()
                            && (c.is_alphanumeric() || c == '_')
                        {
                            self.next_char();
                        }

                        let end_location = self.location;
                        match &self.source[start_location.position..end_location.position] {
                            "normalize" => TokenKind::NormalizeKeyword,
                            "magnitude" => TokenKind::MagnitudeKeyword,
                            "sin" => TokenKind::SinKeyword,
                            "cos" => TokenKind::CosKeyword,
                            "asin" => TokenKind::ASinKeyword,
                            "acos" => TokenKind::ACosKeyword,
                            name => TokenKind::Name(name),
                        }
                    }

                    Some(c) if c.is_numeric() => {
                        while let Some(c) = self.peek_char()
                            && (c.is_numeric() || c == '.')
                        {
                            self.next_char();
                        }

                        let end_location = self.location;
                        let value = self.source[start_location.position..end_location.position]
                            .parse()
                            .map_err(|_| LexerError {
                                location: start_location,
                                kind: LexerErrorKind::InvalidNumber,
                            })?;
                        TokenKind::Number(value)
                    }

                    Some(c) if c.is_whitespace() => continue,
                    Some(c) => {
                        return Err(LexerError {
                            location: start_location,
                            kind: LexerErrorKind::UnexpectedChar(c),
                        });
                    }
                },
            }));
        }
    }

    pub fn peek_token(&self) -> Result<Option<Token<'source>>, LexerError> {
        self.clone().next_token()
    }
}
