use super::token::{Token, TokenKind};

pub struct Scanner<'a> {
    source: &'a str,
    iter: std::str::CharIndices<'a>,
    lookahead: [Option<(usize, char)>; 2],
    start_offset: usize,
    curr_offset: usize,
    curr_line: usize,
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
        }
    }

    pub fn scan_token(&mut self) -> Token<'a> {
        if let Some(err) = self.skip_whitespace() {
            return err;
        }

        self.start_offset = self.curr_offset;

        let c = self.advance();

        if c.is_none() {
            return self.make_token(TokenKind::Eof);
        }

        let c = c.unwrap();

        match c {
            '(' => self.make_token(TokenKind::LeftParen),
            ')' => self.make_token(TokenKind::RightParen),
            '{' => self.make_token(TokenKind::LeftBrace),
            '}' => self.make_token(TokenKind::RightBrace),
            ';' => self.make_token(TokenKind::Semicolon),
            '?' => self.make_token(TokenKind::Question),
            ':' => self.make_token(TokenKind::Colon),
            ',' => self.make_token(TokenKind::Comma),
            '.' => self.make_token(TokenKind::Dot),
            '-' => match self.peek() {
                Some('=') => {
                    self.advance();

                    self.make_token(TokenKind::MinusEqual)
                }
                Some('-') => {
                    self.advance();

                    self.make_token(TokenKind::MinusMinus)
                }
                _ => self.make_token(TokenKind::Minus),
            },
            '+' => match self.peek() {
                Some('=') => {
                    self.advance();

                    self.make_token(TokenKind::PlusEqual)
                }
                Some('+') => {
                    self.advance();

                    self.make_token(TokenKind::PlusPlus)
                }
                _ => self.make_token(TokenKind::Plus),
            },
            '/' => {
                if let Some('=') = self.peek() {
                    self.advance();

                    self.make_token(TokenKind::SlashEqual)
                } else {
                    self.make_token(TokenKind::Slash)
                }
            }
            '*' => {
                if let Some('=') = self.peek() {
                    self.advance();

                    self.make_token(TokenKind::StarEqual)
                } else {
                    self.make_token(TokenKind::Star)
                }
            }
            '!' => {
                if let Some('=') = self.peek() {
                    self.advance();

                    self.make_token(TokenKind::BangEqual)
                } else {
                    self.make_token(TokenKind::Bang)
                }
            }
            '=' => {
                if let Some('=') = self.peek() {
                    self.advance();

                    self.make_token(TokenKind::EqualEqual)
                } else {
                    self.make_token(TokenKind::Equal)
                }
            }
            '>' => {
                if let Some('=') = self.peek() {
                    self.advance();

                    self.make_token(TokenKind::GreaterEqual)
                } else {
                    self.make_token(TokenKind::Greater)
                }
            }
            '<' => {
                if let Some('=') = self.peek() {
                    self.advance();

                    self.make_token(TokenKind::LessEqual)
                } else {
                    self.make_token(TokenKind::Less)
                }
            }
            '"' => self.scan_string(),
            c if c.is_digit(10) => self.scan_number(),
            c if Self::is_alpha(c) => self.scan_identifier(),
            _ => self.make_error_token("Unexpected char"),
        }
    }

    fn advance(&mut self) -> Option<char> {
        let head = self.lookahead[0];

        self.lookahead[0] = self.lookahead[1];
        self.lookahead[1] = self.iter.next();

        if let Some((idx, c)) = head {
            self.curr_offset = idx + c.len_utf8();
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

    fn scan_string(&mut self) -> Token<'a> {
        loop {
            match self.peek() {
                Some('"') => {
                    self.advance(); // Consume the closing quote
                    return self.make_token(TokenKind::String);
                }
                Some(c) => {
                    if c == '\n' {
                        self.curr_line += 1;
                    }
                    self.advance();
                }
                None => {
                    return self.make_error_token("Unterminated string");
                }
            }
        }
    }

    fn scan_number(&mut self) -> Token<'a> {
        self.consume_digits();

        // check for decimal point
        if let Some('.') = self.peek() {
            self.advance();

            // optionally consume digits after '.'
            self.consume_digits();
        }

        self.make_token(TokenKind::Number)
    }

    fn scan_identifier(&mut self) -> Token<'a> {
        loop {
            match self.peek() {
                Some(c) if c.is_digit(10) || Self::is_alpha(c) => {
                    self.advance();
                }
                _ => break,
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
        loop {
            match self.peek() {
                Some(c) if c.is_digit(10) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn skip_whitespace(&mut self) -> Option<Token<'a>> {
        loop {
            match self.peek() {
                Some(' ') => {
                    self.advance();
                }
                Some('\t') => {
                    self.advance();
                }
                Some('\r') => {
                    self.advance();
                }
                Some('\n') => {
                    self.curr_line += 1;
                    self.advance();
                }
                Some('/') => {
                    match self.peek_next() {
                        Some('/') => {
                            // consume until end of line
                            loop {
                                match self.peek() {
                                    Some('\n') => {
                                        self.curr_line += 1;
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
                            // consume "/*"
                            self.advance();
                            self.advance();

                            // consume until "*/"
                            'l1: loop {
                                match self.peek() {
                                    Some('*') => match self.peek_next() {
                                        Some('/') => {
                                            self.advance();
                                            self.advance();

                                            break 'l1;
                                        }
                                        Some(_) => {
                                            self.advance();
                                        }
                                        None => {
                                            return Some(self.make_error_token(
                                                "Unterminated multi-line comment",
                                            ))
                                        }
                                    },
                                    Some('\n') => {
                                        self.curr_line += 1;
                                        self.advance();
                                    }
                                    Some(_) => {
                                        self.advance();
                                    }
                                    None => {
                                        return Some(
                                            self.make_error_token(
                                                "Unterminated multi-line comment",
                                            ),
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
        }
    }

    fn make_error_token(&self, err: &'static str) -> Token<'a> {
        Token {
            kind: TokenKind::Error,
            lexeme: err,
            line: self.curr_line,
        }
    }

    fn is_alpha(c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn scanner_tests() {
        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_files")
            .join("scanning");
        let entries = std::fs::read_dir(&base_dir).unwrap();

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
                    } => {
                        tokens.push(token);
                        break;
                    }
                    token => tokens.push(token),
                }
            }

            assert_eq!(expected, format!("{tokens:#?}\n"));

            // // write output to file
            // let mut expected_file = std::fs::File::create(expected_file).unwrap();

            // expected_file.write_all(format!("{tokens:#?}\n").as_bytes());
        }
    }
}
