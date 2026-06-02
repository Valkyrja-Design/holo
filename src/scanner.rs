//! Lexical analysis for the Holo programming language.
//!
//! This module provides a [`Scanner`] that tokenizes Holo source code into a stream
//! of tokens for the parser.

use crate::error::ScanError;
use crate::token::{Token, TokenKind};

/// A lexical analyzer that converts Holo source code into tokens.
///
/// The scanner uses a two-character lookahead buffer to efficiently handle
/// multi-character operators and comments.
pub struct Scanner<'a> {
    source: &'a str,
    iter: std::str::CharIndices<'a>,
    lookahead: [Option<(usize, char)>; 2],
    start_offset: usize,
    curr_offset: usize,
    curr_line: usize,
    /// 1-based column of the next character to be consumed.
    curr_column: usize,
    /// 1-based column of the first character of the token being scanned.
    start_column: usize,
    /// Structured reason for the most recently produced [`TokenKind::Error`] token.
    last_error: Option<ScanError>,
    /// One entry per interpolation currently being scanned. Each entry counts the
    /// number of unmatched `{` seen inside the embedded expression so that the `}`
    /// closing the interpolation can be told apart from braces nested within it.
    interp_stack: Vec<u32>,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut iter = source.char_indices();
        let mut lookahead = [None; 2];

        for slot in &mut lookahead {
            *slot = iter.next();
        }

        Scanner {
            source,
            iter,
            lookahead,
            start_offset: 0,
            curr_offset: 0,
            curr_line: 1,
            curr_column: 1,
            start_column: 1,
            last_error: None,
            interp_stack: Vec::new(),
        }
    }

    /// Returns the structured reason for the most recent error token, if any.
    pub fn take_error(&mut self) -> Option<ScanError> {
        self.last_error.take()
    }

    pub fn scan_token(&mut self) -> Token<'a> {
        if let Some(err) = self.skip_whitespace() {
            return err;
        }

        self.start_offset = self.curr_offset;
        self.start_column = self.curr_column;

        let c = self.advance();

        if c.is_none() {
            return self.make_token(TokenKind::Eof);
        }

        let c = c.unwrap();

        match c {
            // Single-character tokens
            '(' => self.make_token(TokenKind::LeftParen),
            ')' => self.make_token(TokenKind::RightParen),
            '{' => {
                // Track brace nesting inside an interpolated expression so the
                // matching '}' is not mistaken for the end of the interpolation.
                if let Some(depth) = self.interp_stack.last_mut() {
                    *depth += 1;
                }
                self.make_token(TokenKind::LeftBrace)
            }
            '}' => match self.interp_stack.last_mut() {
                // A '}' at depth 0 ends the current interpolation and resumes
                // scanning the surrounding string literal.
                Some(0) => {
                    self.interp_stack.pop();
                    self.scan_string_body(true)
                }
                Some(depth) => {
                    *depth -= 1;
                    self.make_token(TokenKind::RightBrace)
                }
                None => self.make_token(TokenKind::RightBrace),
            },
            ';' => self.make_token(TokenKind::Semicolon),
            '?' => self.make_token(TokenKind::Question),
            ':' => self.make_token(TokenKind::Colon),
            ',' => self.make_token(TokenKind::Comma),
            '.' => self.make_token(TokenKind::Dot),

            // Multi-character operators
            '-' => self.scan_compound_operator(
                [('=', TokenKind::MinusEqual), ('-', TokenKind::MinusMinus)],
                TokenKind::Minus,
            ),
            '+' => self.scan_compound_operator(
                [('=', TokenKind::PlusEqual), ('+', TokenKind::PlusPlus)],
                TokenKind::Plus,
            ),
            '/' => self.scan_compound_operator([('=', TokenKind::SlashEqual)], TokenKind::Slash),
            '*' => self.scan_compound_operator([('=', TokenKind::StarEqual)], TokenKind::Star),
            '!' => self.scan_compound_operator([('=', TokenKind::BangEqual)], TokenKind::Bang),
            '=' => self.scan_compound_operator([('=', TokenKind::EqualEqual)], TokenKind::Equal),
            '>' => {
                self.scan_compound_operator([('=', TokenKind::GreaterEqual)], TokenKind::Greater)
            }
            '<' => self.scan_compound_operator([('=', TokenKind::LessEqual)], TokenKind::Less),

            // Literals
            '"' => self.scan_string(),
            c if c.is_ascii_digit() => self.scan_number(),
            c if Self::is_identifier_start(c) => self.scan_identifier(),

            _ => self.make_error_token(ScanError::UnexpectedChar(c)),
        }
    }

    fn advance(&mut self) -> Option<char> {
        let head = self.lookahead[0];

        self.lookahead[0] = self.lookahead[1];
        self.lookahead[1] = self.iter.next();

        if let Some((idx, c)) = head {
            self.curr_offset = idx + c.len_utf8();
            if c == '\n' {
                self.curr_line += 1;
                self.curr_column = 1;
            } else {
                self.curr_column += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.lookahead[0].map(|(_, c)| c)
    }

    fn peek_next(&mut self) -> Option<char> {
        self.lookahead[1].map(|(_, c)| c)
    }

    fn scan_compound_operator<const N: usize>(
        &mut self,
        compounds: [(char, TokenKind); N],
        fallback: TokenKind,
    ) -> Token<'a> {
        if let Some(next_char) = self.peek() {
            for (expected, token_kind) in compounds {
                if next_char == expected {
                    self.advance();
                    return self.make_token(token_kind);
                }
            }
        }
        self.make_token(fallback)
    }

    fn scan_string(&mut self) -> Token<'a> {
        self.scan_string_body(false)
    }

    /// Scans the body of a string literal starting just after an opening
    /// delimiter (either the opening `"` or the `}` closing an interpolation).
    ///
    /// `continuation` is true when resuming after an interpolated expression, so
    /// that the chunk is tagged as a continuation/end chunk rather than an
    /// opening one. The literal ends at a closing `"` (yielding a `String` or
    /// `StringInterpEnd` token) or is interrupted by a `{` (yielding a
    /// `StringInterp` or `StringInterpCont` token), in which case the embedded
    /// expression is scanned as ordinary tokens and resumed at the matching `}`.
    fn scan_string_body(&mut self, continuation: bool) -> Token<'a> {
        loop {
            match self.peek() {
                Some('"') => {
                    self.advance(); // Consume the closing quote
                    let kind = if continuation {
                        TokenKind::StringInterpEnd
                    } else {
                        TokenKind::String
                    };
                    return self.make_token(kind);
                }
                Some('{') => {
                    self.advance(); // Consume the opening brace
                    self.interp_stack.push(0);
                    let kind = if continuation {
                        TokenKind::StringInterpCont
                    } else {
                        TokenKind::StringInterp
                    };
                    return self.make_token(kind);
                }
                Some(_) => {
                    self.advance();
                }
                None => {
                    return self.make_error_token(ScanError::UnterminatedString);
                }
            }
        }
    }

    fn scan_number(&mut self) -> Token<'a> {
        self.consume_digits();

        // Check for decimal point
        if let Some('.') = self.peek() {
            self.advance();

            // Optionally consume digits after '.'
            self.consume_digits();
        }

        self.make_token(TokenKind::Number)
    }

    fn scan_identifier(&mut self) -> Token<'a> {
        while let Some(c) = self.peek() {
            if Self::is_identifier_continue(c) {
                self.advance();
            } else {
                break;
            }
        }

        self.make_token(self.resolve_identifier_kind())
    }

    fn resolve_identifier_kind(&self) -> TokenKind {
        let identifier = &self.source[self.start_offset..self.curr_offset];

        match identifier {
            "and" => TokenKind::And,
            "break" => TokenKind::Break,
            "class" => TokenKind::Class,
            "continue" => TokenKind::Continue,
            "else" => TokenKind::Else,
            "false" => TokenKind::False,
            "for" => TokenKind::For,
            "fun" => TokenKind::Fun,
            "if" => TokenKind::If,
            "nil" => TokenKind::Nil,
            "or" => TokenKind::Or,
            "print" => TokenKind::Print,
            "return" => TokenKind::Return,
            "super" => TokenKind::Super,
            "this" => TokenKind::This,
            "true" => TokenKind::True,
            "var" => TokenKind::Var,
            "while" => TokenKind::While,
            _ => TokenKind::Identifier, // Default to Identifier
        }
    }

    fn consume_digits(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_whitespace(&mut self) -> Option<Token<'a>> {
        loop {
            match self.peek()? {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' => {
                    match self.peek_next() {
                        Some('/') => {
                            // Consume until end of line
                            loop {
                                match self.peek() {
                                    Some('\n') => {
                                        self.advance();
                                        break;
                                    }
                                    Some(_) => {
                                        self.advance();
                                    }
                                    None => return None,
                                }
                            }
                        }
                        Some('*') => {
                            // Consume "/*"
                            self.advance();
                            self.advance();

                            // Consume until "*/"
                            loop {
                                match self.peek() {
                                    Some('*') => {
                                        match self.peek_next() {
                                            Some('/') => {
                                                self.advance();
                                                self.advance();
                                                break;
                                            }
                                            Some(_) => {
                                                self.advance();
                                            }
                                            None => {
                                                return Some(self.make_error_token(
                                                    ScanError::UnterminatedComment,
                                                ))
                                            }
                                        }
                                    }
                                    Some(_) => {
                                        self.advance();
                                    }
                                    None => {
                                        return Some(
                                            self.make_error_token(ScanError::UnterminatedComment),
                                        )
                                    }
                                }
                            }
                        }
                        _ => return None,
                    }
                }
                _ => return None,
            }
        }
    }

    fn make_token(&self, kind: TokenKind) -> Token<'a> {
        Token {
            kind,
            lexeme: &self.source[self.start_offset..self.curr_offset],
            line: self.curr_line,
            column: self.start_column,
        }
    }

    fn make_error_token(&mut self, error: ScanError) -> Token<'a> {
        self.last_error = Some(error);
        Token {
            kind: TokenKind::Error,
            lexeme: &self.source[self.start_offset..self.curr_offset],
            line: self.curr_line,
            column: self.start_column,
        }
    }

    fn is_identifier_start(c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }

    fn is_identifier_continue(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn scanner_tests() {
        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("test_files")
            .join("scanning");

        let Ok(entries) = std::fs::read_dir(&base_dir) else {
            panic!("Could not read test directory: {}", base_dir.display());
        };

        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            let expected_file = path.file_stem().unwrap();
            let expected_file = path.parent().unwrap().join("expected").join(expected_file);
            let expected = std::fs::read_to_string(expected_file).unwrap();
            let source = std::fs::read_to_string(path).unwrap();

            let mut scanner = Scanner::new(&source);
            let mut tokens = Vec::new();

            loop {
                match scanner.scan_token() {
                    token @ Token {
                        kind: TokenKind::Eof,
                        lexeme: _,
                        line: _,
                        column: _,
                    } => {
                        tokens.push(token);
                        break;
                    }
                    token => tokens.push(token),
                }
            }

            let normalized_expected = expected.trim().replace("\r\n", "\n");
            let normalized_output = format!("{tokens:#?}\n").trim().replace("\r\n", "\n");

            assert_eq!(normalized_output, normalized_expected);
        }
    }
}
