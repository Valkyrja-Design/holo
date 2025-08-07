use super::{chunk, scanner, token};

pub struct Compiler<'a> {
    source: &'a str,
    scanner: scanner::Scanner<'a>,
    curr_token: token::Token<'a>,
    prev_token: token::Token<'a>,
    had_error: bool,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        // initialize with dummy tokens
        Compiler {
            source,
            scanner: scanner::Scanner::new(source),
            curr_token: token::Token {
                kind: token::TokenKind::Eof,
                lexeme: "",
                line: 0,
            },
            prev_token: token::Token {
                kind: token::TokenKind::Eof,
                lexeme: "",
                line: 0,
            },
            had_error: false,
        }
    }

    pub fn compile(&mut self) -> Option<chunk::Chunk> {
        let mut chunk = chunk::Chunk::new();

        self.advance();
        self.expression();

        self.consume(token::TokenKind::Eof, "Expected end of expression");
        self.finish(&mut chunk);

        if !self.had_error {
            Some(chunk)
        } else {
            None
        }
    }

    fn expression(&mut self) -> Option<String> {
        None
    }

    fn advance(&mut self) {
        self.prev_token = self.curr_token.clone();

        loop {
            match self.scanner.scan_token() {
                token @ token::Token {
                    kind: token::TokenKind::Error,
                    lexeme: err,
                    line: _,
                } => self.report_err(&token, err),
                token => {
                    self.curr_token = token;
                    break;
                }
            }
        }
    }

    fn consume(&mut self, expected: token::TokenKind, err: &'a str) {
        if self.curr_token.kind == expected {
            self.advance();
        } else {
            self.report_err(&self.curr_token.clone(), err);
        }
    }

    fn finish(&self, chunk: &mut chunk::Chunk) {
        self.emit_return(chunk);
    }

    fn emit_byte(&self, chunk: &mut chunk::Chunk, byte: u8) {
        chunk.write_byte(byte, self.prev_token.line);
    }

    fn emit_bytes(&self, chunk: &mut chunk::Chunk, bytes: &[u8], lines: &[usize]) {
        chunk.write_bytes(bytes, lines);
    }

    fn emit_return(&self, chunk: &mut chunk::Chunk) {
        chunk.write_opcode(chunk::OpCode::OpReturn, self.prev_token.line);
    }

    fn report_err(&mut self, token: &token::Token, err: &'a str) {
        self.had_error = true;
        eprintln!("[line {}] Error", token.line);

        match token.kind {
            token::TokenKind::Eof => eprint!(" at end of file"),
            token::TokenKind::Error => {}
            _ => eprint!(" at '{}'", token.lexeme),
        }

        eprintln!(": {}", err);
    }
}
