use super::{chunk, scanner, token, value};

pub struct Compiler<'a> {
    source: &'a str,
    scanner: scanner::Scanner<'a>,
    curr_token: token::Token<'a>,
    prev_token: token::Token<'a>,
    chunk: chunk::Chunk,
    had_error: bool,
}

struct CompileError<'a> {
    token: token::Token<'a>,
    err: String,
}

impl<'a> CompileError<'a> {
    pub fn new(token: token::Token<'a>, err: String) -> Self {
        CompileError { token, err }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Precedence {
    fn as_usize(self) -> usize {
        self as usize
    }
}

impl From<usize> for Precedence {
    fn from(value: usize) -> Self {
        match value {
            0 => Precedence::None,
            1 => Precedence::Assignment,
            2 => Precedence::Or,
            3 => Precedence::And,
            4 => Precedence::Equality,
            5 => Precedence::Comparison,
            6 => Precedence::Term,
            7 => Precedence::Factor,
            8 => Precedence::Unary,
            9 => Precedence::Call,
            _ => Precedence::Primary,
        }
    }
}

impl std::ops::Add<usize> for Precedence {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::Output::from(self.as_usize() + rhs)
    }
}

struct ParseRule<'a> {
    prefix_rule: Option<fn(&mut Compiler<'a>) -> Result<(), CompileError<'a>>>,
    infix_rule: Option<fn(&mut Compiler<'a>) -> Result<(), CompileError<'a>>>,
    precedence: Precedence,
}

impl<'a> Compiler<'a> {
    const RULES: [ParseRule<'a>; 50] = [
        ParseRule {
            prefix_rule: Some(Self::grouping),
            infix_rule: None,
            precedence: Precedence::Primary,
        }, // LeftParen
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // RightParen
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // LeftBrace
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // RightBrace
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Semicolon
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Question
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Colon
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Comma
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Dot
        ParseRule {
            prefix_rule: Some(Self::unary),
            infix_rule: Some(Self::binary),
            precedence: Precedence::Term,
        }, // Minus
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Term,
        }, // Plus
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Factor,
        }, // Slash
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Factor,
        }, // Star
        ParseRule {
            prefix_rule: Some(Self::unary),
            infix_rule: None,
            precedence: Precedence::Unary,
        }, // Bang
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // BangEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Equal
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // EqualEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Greater
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // GreaterEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Less
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // LessEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // PlusPlus
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // PlusEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // MinusMinus
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // MinusEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // StarEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // SlashEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Identifier
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // String
        ParseRule {
            prefix_rule: Some(Self::number),
            infix_rule: None,
            precedence: Precedence::Primary,
        }, // Number
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // And
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Class
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Else
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // False
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Fun
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // For
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // If
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Nil
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Or
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Print
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Return
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Super
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // This
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // True
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Var
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // While
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Break
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Continue
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Error
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Eof
    ];

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
            chunk: chunk::Chunk::new(),
            had_error: false,
        }
    }

    pub fn compile(&mut self) -> Option<&chunk::Chunk> {
        if let Err(err) = self.advance() {
            self.report_err(err);
        }

        if let Err(err) = self.expression() {
            self.report_err(err);
        }

        if let Err(err) = self.consume(token::TokenKind::Eof, "Expected end of expression") {
            self.report_err(err);
        }

        self.finish();

        if !self.had_error {
            Some(&self.chunk)
        } else {
            None
        }
    }

    fn expression(&mut self) -> Result<(), CompileError<'a>> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn number(&mut self) -> Result<(), CompileError<'a>> {
        match self.prev_token.lexeme.parse::<f64>() {
            Ok(value) => self.emit_constant(value),
            Err(err) => Err(CompileError::new(self.prev_token.clone(), err.to_string())),
        }
    }

    fn grouping(&mut self) -> Result<(), CompileError<'a>> {
        self.expression()?;
        self.consume(token::TokenKind::RightParen, "Expected ')'")
    }

    fn unary(&mut self) -> Result<(), CompileError<'a>> {
        let operator_kind = self.prev_token.kind;

        // compile the operand
        self.parse_precedence(Precedence::Unary)?;

        // emit the operator instruction
        match operator_kind {
            token::TokenKind::Minus => self.emit_opcode(chunk::OpCode::Negate),
            _ => {
                return Err(CompileError::new(
                    self.prev_token.clone(),
                    "Unexpected unary operator".to_string(),
                ))
            }
        }

        Ok(())
    }

    fn binary(&mut self) -> Result<(), CompileError<'a>> {
        let operator_token = self.prev_token.clone();
        let operator_kind = self.prev_token.kind;

        // compile the operand
        self.parse_precedence(self.get_rule(operator_kind).precedence + 1)?;

        // emit the operator instruction
        match operator_kind {
            token::TokenKind::Plus => self.emit_opcode(chunk::OpCode::Add),
            token::TokenKind::Minus => self.emit_opcode(chunk::OpCode::Sub),
            token::TokenKind::Star => self.emit_opcode(chunk::OpCode::Mult),
            token::TokenKind::Slash => self.emit_opcode(chunk::OpCode::Divide),
            _ => {
                return Err(CompileError::new(
                    operator_token,
                    "Unexpected binary operator".to_string(),
                ))
            }
        }

        Ok(())
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError<'a>> {
        self.advance()?;

        let prefix_rule = self.get_rule(self.prev_token.kind).prefix_rule;

        match prefix_rule {
            Some(prefix_rule) => prefix_rule(self)?,
            None => {
                return Err(CompileError::new(
                    self.prev_token.clone(),
                    "Expected expression".to_string(),
                ))
            }
        }

        while precedence <= self.get_rule(self.curr_token.kind).precedence {
            self.advance()?;

            let infix_rule = self.get_rule(self.prev_token.kind).infix_rule;

            match infix_rule {
                Some(infix_rule) => infix_rule(self)?,
                None => {
                    return Err(CompileError::new(
                        self.prev_token.clone(),
                        "Expected expression".to_string(),
                    ))
                }
            }
        }

        Ok(())
    }

    fn advance(&mut self) -> Result<(), CompileError<'a>> {
        self.prev_token = self.curr_token.clone();

        match self.scanner.scan_token() {
            token::Token {
                kind: token::TokenKind::Error,
                lexeme: err,
                line: _,
            } => Err(CompileError::new(self.prev_token.clone(), err.to_string())),
            token => {
                self.curr_token = token;
                Ok(())
            }
        }
    }

    fn consume(
        &mut self,
        expected: token::TokenKind,
        err: &'a str,
    ) -> Result<(), CompileError<'a>> {
        if self.curr_token.kind == expected {
            self.advance()
        } else {
            Err(CompileError::new(self.curr_token.clone(), err.to_string()))
        }
    }

    fn finish(&mut self) {
        self.emit_return();
    }

    fn emit_byte(&mut self, byte: u8) {
        self.chunk.write_byte(byte, self.prev_token.line);
    }

    fn emit_opcode(&mut self, opcode: chunk::OpCode) {
        self.chunk.write_opcode(opcode, self.prev_token.line);
    }

    fn emit_return(&mut self) {
        self.chunk
            .write_opcode(chunk::OpCode::Return, self.prev_token.line);
    }

    fn emit_constant(&mut self, value: value::Value) -> Result<(), CompileError<'a>> {
        const MAX24BIT: usize = (1 << 24) - 1;

        let idx = self.chunk.add_constant(value);

        if idx <= u8::MAX as usize {
            self.emit_opcode(chunk::OpCode::Constant);
            self.emit_byte(idx as u8);
            Ok(())
        } else if idx <= MAX24BIT {
            self.emit_opcode(chunk::OpCode::ConstantLong);
            self.chunk.write_as_24bit_int(idx, self.prev_token.line);
            Ok(())
        } else {
            Err(CompileError::new(
                self.prev_token.clone(),
                "Too many constants in the chunk".to_string(),
            ))
        }
    }

    fn get_rule(&self, kind: token::TokenKind) -> &ParseRule<'a> {
        &Self::RULES[kind.as_usize()]
    }

    fn report_err(&mut self, err: CompileError<'a>) {
        self.had_error = true;
        eprintln!("[line {}] Error", err.token.line);

        match err.token.kind {
            token::TokenKind::Eof => eprint!(" at end of file"),
            token::TokenKind::Error => {}
            _ => eprint!(" at '{}'", err.token.lexeme),
        }

        eprintln!(": {}", err.err);
    }
}
