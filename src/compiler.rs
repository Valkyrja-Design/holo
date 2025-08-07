use super::{
    chunk::{Chunk, OpCode},
    gc::GC,
    scanner::Scanner,
    sym_table::SymbolTable,
    table::StringInternTable,
    token::{Token, TokenKind},
    value::Value,
};
use std::io::Write;

struct CompileError<'a> {
    token: Token<'a>,
    err: String,
}

impl<'a> CompileError<'a> {
    pub fn new(token: Token<'a>, err: String) -> Self {
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

struct Local<'a> {
    name: &'a str,
    depth: i32,
}

pub struct Compiler<'a, 'b, W: Write> {
    source: &'a str,
    scanner: Scanner<'a>,
    curr_token: Token<'a>,
    prev_token: Token<'a>,
    chunk: Chunk,
    gc: &'b mut GC,
    str_intern_table: &'b mut StringInternTable,
    sym_table: SymbolTable<'a>,
    locals: Vec<Local<'a>>,
    curr_depth: i32,
    had_error: bool,
    err_stream: &'b mut W,
}

struct ParseRule<'a, 'b, W: Write> {
    prefix_rule: Option<fn(&mut Compiler<'a, 'b, W>, bool) -> Result<(), CompileError<'a>>>,
    infix_rule: Option<fn(&mut Compiler<'a, 'b, W>, bool) -> Result<(), CompileError<'a>>>,
    precedence: Precedence,
}

impl<'a, 'b, W: Write> Compiler<'a, 'b, W> {
    const RULES: [ParseRule<'a, 'b, W>; 50] = [
        ParseRule {
            prefix_rule: Some(Self::grouping),
            infix_rule: None,
            precedence: Precedence::None,
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
            infix_rule: Some(Self::ternary),
            precedence: Precedence::Assignment,
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
            precedence: Precedence::None,
        }, // Bang
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Equality,
        }, // BangEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: None,
            precedence: Precedence::None,
        }, // Equal
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Equality,
        }, // EqualEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Comparison,
        }, // Greater
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Comparison,
        }, // GreaterEqual
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Comparison,
        }, // Less
        ParseRule {
            prefix_rule: None,
            infix_rule: Some(Self::binary),
            precedence: Precedence::Comparison,
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
            prefix_rule: Some(Self::variable),
            infix_rule: None,
            precedence: Precedence::None,
        }, // Identifier
        ParseRule {
            prefix_rule: Some(Self::string),
            infix_rule: None,
            precedence: Precedence::None,
        }, // String
        ParseRule {
            prefix_rule: Some(Self::number),
            infix_rule: None,
            precedence: Precedence::None,
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
            prefix_rule: Some(Self::literal),
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
            prefix_rule: Some(Self::literal),
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
            prefix_rule: Some(Self::literal),
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

    pub fn new(
        source: &'a str,
        gc: &'b mut GC,
        str_intern_table: &'b mut StringInternTable,
        err_stream: &'b mut W,
    ) -> Self {
        // initialize with dummy tokens
        Compiler {
            source,
            scanner: Scanner::new(source),
            curr_token: Token {
                kind: TokenKind::Eof,
                lexeme: "",
                line: 0,
            },
            prev_token: Token {
                kind: TokenKind::Eof,
                lexeme: "",
                line: 0,
            },
            chunk: Chunk::new(),
            gc,
            str_intern_table,
            sym_table: SymbolTable::new(),
            locals: Vec::new(),
            curr_depth: 0,
            had_error: false,
            err_stream,
        }
    }

    pub fn compile(mut self) -> Option<(Chunk, SymbolTable<'a>)> {
        if let Err(err) = self.advance() {
            self.report_err(err);

            // synchronize the parser state
            self.synchronize();
        }

        while !self.check(TokenKind::Eof) {
            if let Err(err) = self.declaration() {
                self.report_err(err);

                // synchronize the parser state
                self.synchronize();
            }
        }

        self.finish();

        if !self.had_error {
            Some((self.chunk, self.sym_table))
        } else {
            None
        }
    }

    fn declaration(&mut self) -> Result<(), CompileError<'a>> {
        match self.curr_token.kind {
            TokenKind::Var => {
                self.advance()?;
                self.var_declaration()
            }
            _ => self.statement(),
        }
    }

    fn var_declaration(&mut self) -> Result<(), CompileError<'a>> {
        self.consume(TokenKind::Identifier, "Expected variable name")?;

        let name = self.prev_token.lexeme;
        let index;

        if self.curr_depth > 0 {
            index = self.declare_local(name)?;
        } else {
            index = self.sym_table.declare(name);
        }

        // consume the initializer, if any
        if self.check(TokenKind::Equal) {
            self.advance()?;
            self.expression()?;
        } else {
            self.emit_opcode(OpCode::Nil);
        }

        self.consume(TokenKind::Semicolon, "Expected ';' at the end of statement")?;

        if self.curr_depth > 0 {
            self.mark_as_initialized(index);
            Ok(())
        } else {
            self.emit_opcode_with_num(
                OpCode::DefineGlobal,
                OpCode::DefineGlobalLong,
                index,
                "Too many globals in the program".to_owned(),
            )
        }
    }

    fn statement(&mut self) -> Result<(), CompileError<'a>> {
        match self.curr_token.kind {
            TokenKind::Print => {
                self.advance()?;
                self.print_statement()
            }
            TokenKind::LeftBrace => {
                self.advance()?;
                self.begin_scope();
                self.block()?;
                self.end_scope();
                Ok(())
            }
            TokenKind::If => {
                self.advance()?;
                self.if_stmt()
            }
            _ => self.expression_statement(),
        }
    }

    fn print_statement(&mut self) -> Result<(), CompileError<'a>> {
        self.expression()?;
        self.consume(TokenKind::Semicolon, "Expected ';' at the end of statement")?;
        self.emit_opcode(OpCode::Print);

        Ok(())
    }

    fn block(&mut self) -> Result<(), CompileError<'a>> {
        loop {
            match self.curr_token.kind {
                TokenKind::RightBrace => {
                    self.advance()?;
                    return Ok(());
                }
                TokenKind::Eof => {
                    return Err(CompileError::new(
                        self.curr_token.to_owned(),
                        "Expected closing '}' for the block".to_owned(),
                    ))
                }
                _ => self.declaration()?,
            }
        }
    }

    fn if_stmt(&mut self) -> Result<(), CompileError<'a>> {
        self.consume(TokenKind::LeftParen, "Expected '('")?;
        // compile the condition
        self.expression()?;
        self.consume(TokenKind::RightParen, "Expected ')'")?;

        let then_jump = self.emit_jump(OpCode::JumpIfFalse);

        // pop the condition
        self.emit_opcode(OpCode::Pop);
        // compile the block
        self.statement()?;

        // to skip the `else` block after executing the `if` block
        let else_jump = self.emit_jump(OpCode::Jump);

        // `else` branch starts now
        self.patch_jump(then_jump)?;

        // pop the condition in the `else` branch
        self.emit_opcode(OpCode::Pop);

        // compile the `else` branch if present
        if self.check(TokenKind::Else) {
            self.advance()?;
            self.statement()?;
        }

        // end of `else` branch
        self.patch_jump(else_jump)
    }

    fn expression_statement(&mut self) -> Result<(), CompileError<'a>> {
        self.expression()?;
        self.consume(TokenKind::Semicolon, "Expected ';' at the end of statement")?;
        self.emit_opcode(OpCode::Pop);

        Ok(())
    }

    fn expression(&mut self) -> Result<(), CompileError<'a>> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn variable(&mut self, can_assign: bool) -> Result<(), CompileError<'a>> {
        let name = self.prev_token.lexeme;
        let local_idx = self.resolve_local(name);

        // pick local or global ops and final index
        let (get_op, get_op_long, set_op, set_op_long, idx) = if local_idx != -1 {
            if self.locals[local_idx as usize].depth == -1 {
                return Err(CompileError::new(
                    self.prev_token.to_owned(),
                    format!("Cannot use variable '{}' in its own initializer", name),
                ));
            }

            (
                OpCode::GetLocal,
                OpCode::GetLocalLong,
                OpCode::SetLocal,
                OpCode::SetLocalLong,
                local_idx as usize,
            )
        } else {
            let global_idx = self.sym_table.resolve(name);

            (
                OpCode::GetGlobal,
                OpCode::GetGlobalLong,
                OpCode::SetGlobal,
                OpCode::SetGlobalLong,
                global_idx,
            )
        };

        // assignment or read
        if can_assign && self.curr_token.kind == TokenKind::Equal {
            self.advance()?;
            self.expression()?;
            self.emit_opcode_with_num(
                set_op,
                set_op_long,
                idx,
                "Too many globals in the program".to_string(),
            )
        } else {
            self.emit_opcode_with_num(
                get_op,
                get_op_long,
                idx,
                "Too many globals in the program".to_string(),
            )
        }
    }

    fn number(&mut self, _: bool) -> Result<(), CompileError<'a>> {
        match self.prev_token.lexeme.parse::<f64>() {
            Ok(value) => self.emit_opcode_with_constant(
                OpCode::Constant,
                OpCode::ConstantLong,
                Value::Number(value),
            ),
            Err(err) => Err(CompileError::new(self.prev_token.clone(), err.to_string())),
        }
    }

    fn literal(&mut self, _: bool) -> Result<(), CompileError<'a>> {
        match self.prev_token.kind {
            TokenKind::Nil => {
                self.emit_opcode(OpCode::Nil);
                Ok(())
            }
            TokenKind::True => {
                self.emit_opcode(OpCode::True);
                Ok(())
            }
            TokenKind::False => {
                self.emit_opcode(OpCode::False);
                Ok(())
            }
            _ => Err(CompileError::new(
                self.prev_token.clone(),
                "Expected a literal".to_string(),
            )),
        }
    }

    fn string(&mut self, _: bool) -> Result<(), CompileError<'a>> {
        match self.prev_token.kind {
            TokenKind::String => {
                let s = &self.prev_token.lexeme[1..self.prev_token.lexeme.len() - 1];
                let str_ptr = self.str_intern_table.intern_slice(s, self.gc);

                self.emit_opcode_with_constant(
                    OpCode::Constant,
                    OpCode::ConstantLong,
                    Value::Object(str_ptr),
                )
            }
            _ => Err(CompileError::new(
                self.prev_token.clone(),
                "Expected a string".to_string(),
            )),
        }
    }

    fn grouping(&mut self, _: bool) -> Result<(), CompileError<'a>> {
        self.expression()?;
        self.consume(TokenKind::RightParen, "Expected ')'")
    }

    fn unary(&mut self, _: bool) -> Result<(), CompileError<'a>> {
        let operator_kind = self.prev_token.kind;

        // compile the operand
        self.parse_precedence(Precedence::Unary)?;

        // emit the operator instruction
        match operator_kind {
            TokenKind::Minus => self.emit_opcode(OpCode::Negate),
            TokenKind::Bang => self.emit_opcode(OpCode::Not),
            _ => {
                return Err(CompileError::new(
                    self.prev_token.clone(),
                    "Unexpected unary operator".to_string(),
                ))
            }
        }

        Ok(())
    }

    fn binary(&mut self, _: bool) -> Result<(), CompileError<'a>> {
        let operator_token = self.prev_token.clone();
        let operator_kind = self.prev_token.kind;

        // compile the operand
        self.parse_precedence(self.get_rule(operator_kind).precedence + 1)?;

        // emit the operator instruction
        match operator_kind {
            TokenKind::Plus => self.emit_opcode(OpCode::Add),
            TokenKind::Minus => self.emit_opcode(OpCode::Sub),
            TokenKind::Star => self.emit_opcode(OpCode::Mult),
            TokenKind::Slash => self.emit_opcode(OpCode::Divide),
            TokenKind::EqualEqual => self.emit_opcode(OpCode::Equal),
            TokenKind::BangEqual => self.emit_opcode(OpCode::NotEqual),
            TokenKind::Greater => self.emit_opcode(OpCode::Greater),
            TokenKind::GreaterEqual => self.emit_opcode(OpCode::GreaterEqual),
            TokenKind::Less => self.emit_opcode(OpCode::Less),
            TokenKind::LessEqual => self.emit_opcode(OpCode::LessEqual),
            _ => {
                return Err(CompileError::new(
                    operator_token,
                    "Unexpected binary operator".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn ternary(&mut self, _: bool) -> Result<(), CompileError<'a>> {
        let operator_kind = self.prev_token.kind;

        if let TokenKind::Question = operator_kind {
            // compile the 2nd operand
            self.parse_precedence(Precedence::Assignment)?;

            // consume the colon
            self.consume(TokenKind::Colon, "Expected ':'")?;

            // compile the 3rd operand
            self.parse_precedence(Precedence::Assignment)?;

            self.emit_opcode(OpCode::Ternary);
            Ok(())
        } else {
            Err(CompileError::new(
                self.prev_token.clone(),
                "Expected '?'".to_string(),
            ))
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError<'a>> {
        self.advance()?;

        let prefix_rule = self.get_rule(self.prev_token.kind).prefix_rule;
        let can_assign = precedence <= Precedence::Assignment;

        match prefix_rule {
            Some(prefix_rule) => prefix_rule(self, can_assign)?,
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
                Some(infix_rule) => infix_rule(self, can_assign)?,
                None => {
                    return Err(CompileError::new(
                        self.prev_token.clone(),
                        "Expected expression".to_string(),
                    ))
                }
            }
        }

        if can_assign && self.check(TokenKind::Equal) {
            Err(CompileError::new(
                self.curr_token.clone(),
                "Invalid assignment target".to_owned(),
            ))
        } else {
            Ok(())
        }
    }

    fn advance(&mut self) -> Result<(), CompileError<'a>> {
        let token = self.scanner.scan_token();

        self.prev_token = self.curr_token.clone();
        self.curr_token = token.clone();

        if let TokenKind::Error = token.kind {
            return Err(CompileError::new(token.clone(), token.lexeme.to_string()));
        }

        Ok(())
    }

    fn consume(&mut self, expected: TokenKind, err: &'a str) -> Result<(), CompileError<'a>> {
        if self.check(expected) {
            self.advance()
        } else {
            Err(CompileError::new(self.curr_token.clone(), err.to_string()))
        }
    }

    fn synchronize(&mut self) {
        loop {
            match self.curr_token.kind {
                TokenKind::Eof => return,
                TokenKind::For => return,
                TokenKind::If => return,
                TokenKind::While => return,
                TokenKind::Fun => return,
                TokenKind::Var => return,
                TokenKind::Print => return,
                TokenKind::Semicolon => {
                    if let Err(err) = self.advance() {
                        self.report_err(err);
                        continue;
                    }

                    return;
                }
                _ => {
                    if let Err(err) = self.advance() {
                        self.report_err(err);
                    }
                }
            }
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.curr_token.kind == kind
    }

    fn finish(&mut self) {
        self.emit_return();
    }

    /// declare local but set its depth to -1 as a marker for "not initialized"
    fn declare_local(&mut self, name: &'a str) -> Result<usize, CompileError<'a>> {
        for local in self.locals.iter().rev() {
            if local.depth != -1 && local.depth < self.curr_depth {
                break;
            }

            if local.name == name {
                return Err(CompileError::new(
                    self.prev_token.to_owned(),
                    format!("Redeclaration of variable '{}'", name),
                ));
            }
        }

        self.locals.push(Local { name, depth: -1 });

        Ok(self.locals.len() - 1)
    }

    /// mark the local as being initialized by setting the depth to current depth
    fn mark_as_initialized(&mut self, index: usize) {
        self.locals[index].depth = self.curr_depth;
    }

    /// returns the index of the first declaration of the given local, -1 otherwise
    fn resolve_local(&mut self, name: &'a str) -> i32 {
        for (index, local) in self.locals.iter().rev().enumerate() {
            if local.name == name {
                return (self.locals.len() - index - 1) as i32;
            }
        }

        -1
    }

    /// increases the current scope depth
    fn begin_scope(&mut self) {
        self.curr_depth += 1;
    }

    /// decreases the current scope depth and pops off all locals in the current scope
    fn end_scope(&mut self) {
        let mut locals_to_pop = 0;

        while let Some(local) = self.locals.last() {
            if local.depth != self.curr_depth {
                break;
            }

            self.locals.pop();
            locals_to_pop += 1;
        }

        self.curr_depth -= 1;
        // this call won't throw error because declarations would've already done that
        let _ =
            self.emit_opcode_with_num(OpCode::PopN, OpCode::PopNLong, locals_to_pop, "".to_owned());
    }

    fn emit_byte(&mut self, byte: u8) {
        self.chunk.write_byte(byte, self.prev_token.line);
    }

    fn emit_opcode(&mut self, opcode: OpCode) {
        self.chunk.write_opcode(opcode, self.prev_token.line);
    }

    fn emit_return(&mut self) {
        self.chunk
            .write_opcode(OpCode::Return, self.prev_token.line);
    }

    fn emit_opcode_with_num(
        &mut self,
        opcode: OpCode,
        opcode_long: OpCode,
        num: usize,
        err: String,
    ) -> Result<(), CompileError<'a>> {
        const MAX24BIT: usize = (1 << 24) - 1;

        if num <= u8::MAX as usize {
            self.emit_opcode(opcode);
            self.emit_byte(num as u8);
            Ok(())
        } else if num <= MAX24BIT {
            self.emit_opcode(opcode_long);
            self.chunk.write_as_24bit_int(num, self.prev_token.line);
            Ok(())
        } else {
            Err(CompileError::new(self.prev_token.clone(), err))
        }
    }

    fn emit_opcode_with_constant(
        &mut self,
        opcode: OpCode,
        opcode_long: OpCode,
        value: Value,
    ) -> Result<(), CompileError<'a>> {
        let index = self.chunk.add_constant(value);
        self.emit_opcode_with_num(
            opcode,
            opcode_long,
            index,
            "Too many constants in the chunk".to_string(),
        )
    }

    fn emit_jump(&mut self, opcode: OpCode) -> usize {
        self.chunk.write_opcode(opcode, self.prev_token.line);
        self.chunk.write_bytes(&[0; 2], &[self.prev_token.line; 2]);
        // return the location of the first byte of the jump address
        self.chunk.code.len() - 2
    }

    fn patch_jump(&mut self, offset: usize) -> Result<(), CompileError<'a>> {
        const BYTE_MASK: usize = (1usize << 8) - 1;

        let jump_dist = self.chunk.code.len() - offset - 2;

        if jump_dist > u16::MAX as usize {
            Err(CompileError::new(
                self.prev_token.clone(),
                "Too much jump distance".to_string(),
            ))
        } else {
            self.chunk.code[offset] = ((jump_dist >> 8) & BYTE_MASK) as u8;
            self.chunk.code[offset + 1] = (jump_dist & BYTE_MASK) as u8;
            Ok(())
        }
    }

    fn get_rule(&self, kind: TokenKind) -> &ParseRule<'a, 'b, W> {
        &Self::RULES[kind.as_usize()]
    }

    fn report_err(&mut self, err: CompileError<'a>) {
        self.had_error = true;
        write!(self.err_stream, "[line {}] Error", err.token.line).unwrap();
        match err.token.kind {
            TokenKind::Eof => write!(self.err_stream, " at end of file").unwrap(),
            TokenKind::Error => {}
            _ => write!(self.err_stream, " at '{}'", err.token.lexeme).unwrap(),
        }
        writeln!(self.err_stream, ": {}", err.err).unwrap();
    }
}
