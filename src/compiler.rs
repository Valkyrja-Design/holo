use super::{
    chunk::{Chunk, OpCode},
    error::{CompileError, CompileErrorKind, Expected},
    gc::GC,
    scanner::Scanner,
    sym_table::SymbolTable,
    table::StringInternTable,
    token::{Token, TokenKind},
    value::{Function, Value},
};
use std::io::Write;

type Result<'a, T> = std::result::Result<T, CompileError<'a>>;

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
    depth: usize,
    initialized: bool,
    captured: bool,
}

impl<'a> Local<'a> {
    fn new(name: &'a str, depth: usize, initialized: bool, captured: bool) -> Self {
        Local {
            name,
            depth,
            initialized,
            captured,
        }
    }
}

type ParseFn<'a, 'b, W> = fn(&mut Compiler<'a, 'b, W>, bool) -> Result<'a, ()>;

struct ParseRule<'a, 'b, W: Write> {
    prefix_rule: Option<ParseFn<'a, 'b, W>>,
    infix_rule: Option<ParseFn<'a, 'b, W>>,
    precedence: Precedence,
}

struct LoopContext {
    loop_start: usize, // Start offset of the loop bytecode (condition or the update expression)
    scope_depth: usize, // Scope depth at the start of the loop
    break_jumps: Vec<usize>, // Jump statements to patch to the end of the loop
}

struct Upvalue {
    index: usize,
    is_local: bool,
}

struct CompilerContext<'a> {
    function: Function,
    loop_contexts: Vec<LoopContext>,
    curr_depth: usize,
    locals: Vec<Local<'a>>,
    upvalues: Vec<Upvalue>,
    is_initializer: bool,
}

struct ClassContext {
    has_superclass: bool,
}

pub struct Compiler<'a, 'b, W: Write> {
    source: &'a str,
    scanner: Scanner<'a>,
    curr_token: Token<'a>,
    prev_token: Token<'a>,

    // Current compilation context
    function: Function,
    locals: Vec<Local<'a>>,
    curr_depth: usize,
    loop_contexts: Vec<LoopContext>,
    upvalues: Vec<Upvalue>,
    is_initializer: bool,

    // Saved contexts for nested functions
    contexts: Vec<CompilerContext<'a>>,
    class_contexts: Vec<ClassContext>,

    // Shared state
    gc: &'b mut GC,
    str_intern_table: &'b mut StringInternTable,
    sym_table: &'b mut SymbolTable<'a>,
    had_error: bool,
    err_stream: &'b mut W,
}

impl<'a, 'b, W: Write> Compiler<'a, 'b, W> {
    const RULES: [ParseRule<'a, 'b, W>; 50] = [
        ParseRule {
            prefix_rule: Some(Self::grouping),
            infix_rule: Some(Self::call),
            precedence: Precedence::Call,
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
            infix_rule: Some(Self::dot),
            precedence: Precedence::Call,
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
            infix_rule: Some(Self::logical_and),
            precedence: Precedence::And,
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
            infix_rule: Some(Self::logical_or),
            precedence: Precedence::Or,
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
            prefix_rule: Some(Self::super_),
            infix_rule: None,
            precedence: Precedence::None,
        }, // Super
        ParseRule {
            prefix_rule: Some(Self::this),
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
        func_name: &str,
        gc: &'b mut GC,
        str_intern_table: &'b mut StringInternTable,
        sym_table: &'b mut SymbolTable<'a>,
        err_stream: &'b mut W,
    ) -> Self {
        Compiler {
            source,
            scanner: Scanner::new(source),
            curr_token: Token {
                kind: TokenKind::Eof,
                lexeme: "",
                line: 0,
                column: 0,
            },
            prev_token: Token {
                kind: TokenKind::Eof,
                lexeme: "",
                line: 0,
                column: 0,
            },
            function: Function {
                name: func_name.to_owned(),
                arity: 0,
                upvalue_count: 0,
                chunk: Chunk::new(),
            },
            locals: Vec::new(),
            curr_depth: 0,
            loop_contexts: Vec::new(),
            upvalues: Vec::new(),
            is_initializer: false,
            had_error: false,
            contexts: Vec::new(),
            class_contexts: Vec::new(),
            gc,
            str_intern_table,
            sym_table,
            err_stream,
        }
    }

    pub fn compile(mut self) -> Option<Function> {
        if let Err(err) = self.advance() {
            self.report_err(err);

            // Synchronize the parser state
            self.synchronize();
        }

        while !self.check(TokenKind::Eof) {
            if let Err(err) = self.declaration() {
                self.report_err(err);

                // Synchronize the parser state
                self.synchronize();
            }
        }

        self.finish()
    }

    fn declaration(&mut self) -> Result<'a, ()> {
        match self.curr_token.kind {
            TokenKind::Var => {
                self.advance()?;
                self.var_declaration()
            }
            TokenKind::Fun => {
                self.advance()?;
                self.fun_declaration()
            }
            TokenKind::Class => {
                self.advance()?;
                self.class_declaration()
            }
            _ => self.statement(),
        }
    }

    fn var_declaration(&mut self) -> Result<'a, ()> {
        self.consume(TokenKind::Identifier, Expected::VariableName)?;

        let name = self.prev_token.lexeme;
        let index = if self.curr_depth > 0 {
            self.declare_local(name)?
        } else {
            self.sym_table.declare(name)
        };

        // Consume the initializer, if any
        if self.check(TokenKind::Equal) {
            self.advance()?;
            self.expression()?;
        } else {
            self.emit_opcode(OpCode::Nil);
        }

        self.consume(TokenKind::Semicolon, Expected::Semicolon)?;

        if self.curr_depth > 0 {
            self.mark_as_initialized(index);
            Ok(())
        } else {
            self.emit_opcode_with_num(
                OpCode::DefineGlobal,
                OpCode::DefineGlobalLong,
                index,
                CompileErrorKind::TooManyGlobals,
            )
        }
    }

    fn fun_declaration(&mut self) -> Result<'a, ()> {
        self.consume(TokenKind::Identifier, Expected::FunctionName)?;

        let name = self.prev_token.lexeme;

        // Declare the function name in the current scope
        let index = if self.curr_depth > 0 {
            let index = self.declare_local(name)?;
            self.mark_as_initialized(index);
            index
        } else {
            self.sym_table.declare(name)
        };

        // Save the current context
        self.push_context(name, false);
        self.begin_scope();

        // Reserve a slot for the function itself
        let func_index = self.declare_local("")?;
        self.mark_as_initialized(func_index);

        // Compile the function body
        self.function()?;

        // Define it as a variable
        if self.curr_depth > 0 {
            // Local variable
            Ok(())
        } else {
            // Global variable
            self.emit_opcode_with_num(
                OpCode::DefineGlobal,
                OpCode::DefineGlobalLong,
                index,
                CompileErrorKind::TooManyGlobals,
            )
        }
    }

    /// Compiles a function signature and body, assumes the `fun` keyword has been consumed
    /// and a new scope has been created. The caller does not have to explicitly end the scope
    /// because this function will pop the new function's compilation context anyway
    fn function(&mut self) -> Result<'a, ()> {
        const MAX_PARAMS: u8 = 255;

        // Compile the parameter list
        self.consume(TokenKind::LeftParen, Expected::LeftParenAfterFunctionName)?;

        let mut arity: u8 = 0;

        if !self.check(TokenKind::RightParen) {
            loop {
                self.consume(TokenKind::Identifier, Expected::ParameterName)?;

                if arity == MAX_PARAMS {
                    return Err(CompileError::new(
                        self.prev_token.clone(),
                        CompileErrorKind::TooManyParameters,
                    ));
                }

                arity += 1;

                let name = self.prev_token.lexeme;
                let index = self.declare_local(name)?;
                self.mark_as_initialized(index);

                if !self.check(TokenKind::Comma) {
                    break;
                }

                self.advance()?;
            }
        }

        self.function.arity = arity;
        self.consume(TokenKind::RightParen, Expected::RightParenAfterParameters)?;

        // Compile the body
        self.consume(TokenKind::LeftBrace, Expected::LeftBraceBeforeFunctionBody)?;
        self.block()?;

        // Implicit return
        self.emit_return()?;

        // Restore the previous context
        let upvalues = std::mem::take(&mut self.upvalues);
        let mut function = self.pop_context();

        // Fill in the upvalue count
        function.upvalue_count = upvalues.len();

        // Allocate the function value
        let func_value = self.gc.alloc_function(function);

        // Emit a `Closure` instruction to wrap the function at runtime
        self.emit_opcode_with_constant_long(OpCode::Closure, OpCode::ClosureLong, func_value)?;

        // Emit the upvalues
        for upvalue in upvalues {
            // The `Closure` instruction encodes each captured variable's slot
            // index in a single byte, so reject closures that would capture a
            // local slot or enclosing upvalue beyond that range.
            if upvalue.index > u8::MAX as usize {
                return Err(CompileError::new(
                    self.prev_token.clone(),
                    CompileErrorKind::TooManyUpvalues,
                ));
            }

            self.emit_byte(if upvalue.is_local { 1 } else { 0 });
            self.emit_byte(upvalue.index as u8);
        }

        Ok(())
    }

    /// Compiles a class declaration, assumes the `class` keyword has been consumed
    fn class_declaration(&mut self) -> Result<'a, ()> {
        self.consume(TokenKind::Identifier, Expected::ClassName)?;

        let class_name = self.prev_token.lexeme;
        let index = self.declare_variable(class_name)?;

        // FIXME: Add support for u24 constants
        // Emit the `Class` instruction
        let str_ptr = self.str_intern_table.intern_slice(class_name, self.gc);
        self.emit_opcode_with_constant(OpCode::Class, Value::String(str_ptr))?;

        // Define it as a variable
        if self.curr_depth == 0 {
            // Global variable
            self.emit_opcode_with_num(
                OpCode::DefineGlobal,
                OpCode::DefineGlobalLong,
                index,
                CompileErrorKind::TooManyGlobals,
            )?;
        }

        self.class_contexts.push(ClassContext {
            has_superclass: false,
        });

        // Compile the superclass, if any
        if self.check(TokenKind::Colon) {
            self.advance()?;
            self.consume(TokenKind::Identifier, Expected::SuperclassName)?;

            if self.prev_token.lexeme == class_name {
                return Err(CompileError::new(
                    self.prev_token.clone(),
                    CompileErrorKind::InheritFromSelf,
                ));
            }

            // Load the superclass
            self.resolve_variable(self.prev_token.lexeme)?;
            self.class_contexts.last_mut().unwrap().has_superclass = true;

            // Declare a variable for the superclass
            self.begin_scope();
            let index = self.declare_local("super")?;
            self.mark_as_initialized(index);

            // Load the current class. We must declare the superclass variable
            // before loading the current class because the methods expect the
            // current class to be at the top of the stack
            self.resolve_variable(class_name)?;

            // The superclass is at the top of the stack with the class variable below it
            self.emit_opcode(OpCode::Inherit);
        }

        // Push the class variable back onto the stack, so that the methods and the
        // `OpCode::Inherit` can access it
        self.resolve_variable(class_name)?;

        // Compile the class body
        self.consume(TokenKind::LeftBrace, Expected::LeftBraceBeforeClassBody)?;

        // Compile the method declarations
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.method_declaration()?;
        }

        self.consume(TokenKind::RightBrace, Expected::RightBraceAfterClassBody)?;

        // Pop the class variable
        self.emit_opcode(OpCode::Pop);

        // End the class scope if the superclass was present
        if self.class_contexts.last().unwrap().has_superclass {
            self.end_scope();
        }

        self.class_contexts.pop();
        Ok(())
    }

    /// Compiles a method declaration
    fn method_declaration(&mut self) -> Result<'a, ()> {
        // Method declarations don't begin with `fun` keyword
        self.consume(TokenKind::Identifier, Expected::MethodName)?;

        let method_name = self.prev_token.lexeme;
        let method_name_ptr = self.str_intern_table.intern_slice(method_name, self.gc);

        // Save the current context
        self.push_context(method_name, method_name == "init");
        self.begin_scope();

        // Reserver a slot for the `this` variable
        let this_index = self.declare_local("this")?;
        self.mark_as_initialized(this_index);

        // Compile the function body
        self.function()?;

        // At this point the method's closure is on the stack with the
        // parent class directly below it
        self.emit_opcode_with_constant(OpCode::Method, Value::String(method_name_ptr))
    }

    fn statement(&mut self) -> Result<'a, ()> {
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
            TokenKind::While => {
                self.advance()?;
                self.while_stmt()
            }
            TokenKind::For => {
                self.advance()?;
                self.for_stmt()
            }
            TokenKind::Continue => {
                self.advance()?;
                self.continue_stmt()
            }
            TokenKind::Break => {
                self.advance()?;
                self.break_stmt()
            }
            TokenKind::Return => {
                self.advance()?;
                self.return_stmt()
            }
            _ => self.expression_statement(),
        }
    }

    fn print_statement(&mut self) -> Result<'a, ()> {
        self.expression()?;
        self.consume(TokenKind::Semicolon, Expected::Semicolon)?;
        self.emit_opcode(OpCode::Print);

        Ok(())
    }

    fn block(&mut self) -> Result<'a, ()> {
        loop {
            match self.curr_token.kind {
                TokenKind::RightBrace => {
                    self.advance()?;
                    return Ok(());
                }
                TokenKind::Eof => {
                    return Err(CompileError::new(
                        self.curr_token.to_owned(),
                        CompileErrorKind::Expected(Expected::RightBraceToCloseBlock),
                    ))
                }
                _ => self.declaration()?,
            }
        }
    }

    fn if_stmt(&mut self) -> Result<'a, ()> {
        self.consume(TokenKind::LeftParen, Expected::LeftParen)?;
        // Compile the condition
        self.expression()?;
        self.consume(TokenKind::RightParen, Expected::RightParen)?;

        let then_jump = self.emit_jump(OpCode::JumpIfFalse);

        // Pop the condition
        self.emit_opcode(OpCode::Pop);
        // Compile the block
        self.statement()?;

        // To skip the `else` block after executing the `if` block
        let else_jump = self.emit_jump(OpCode::Jump);

        // `else` branch starts now
        self.patch_jump(then_jump)?;

        // Pop the condition in the `else` branch
        self.emit_opcode(OpCode::Pop);

        // Compile the `else` branch if present
        if self.check(TokenKind::Else) {
            self.advance()?;
            self.statement()?;
        }

        // End of `else` branch
        self.patch_jump(else_jump)
    }

    fn while_stmt(&mut self) -> Result<'a, ()> {
        let loop_start = self.chunk().code.len();

        self.begin_loop(loop_start);

        self.consume(TokenKind::LeftParen, Expected::LeftParen)?;
        // Compile the condition
        self.expression()?;
        self.consume(TokenKind::RightParen, Expected::RightParen)?;

        let exit_jump = self.emit_jump(OpCode::JumpIfFalse);

        // Pop the condition
        self.emit_opcode(OpCode::Pop);
        // Compile the body
        self.statement()?;

        self.emit_loop(loop_start)?;
        self.patch_jump(exit_jump)?;

        self.emit_opcode(OpCode::Pop);
        self.end_loop()?;
        Ok(())
    }

    fn for_stmt(&mut self) -> Result<'a, ()> {
        // Start a new scope for the initializer
        self.begin_scope();

        self.consume(TokenKind::LeftParen, Expected::LeftParen)?;

        // Compile the initializer, if any. It can be a variable declaration,
        // expression statement or just ';'
        match self.curr_token.kind {
            TokenKind::Var => {
                self.advance()?;
                self.var_declaration()?;
            }
            TokenKind::Semicolon => {
                self.advance()?;
            }
            _ => {
                self.expression_statement()?;
            }
        }

        let mut loop_start = self.chunk().code.len();
        let mut exit_jump: isize = -1;

        // Compile the condition, if any
        if !self.check(TokenKind::Semicolon) {
            self.expression()?;

            // We have the condition value on top of the stack
            exit_jump = self.emit_jump(OpCode::JumpIfFalse) as isize;
            self.emit_opcode(OpCode::Pop);
        }

        self.consume(TokenKind::Semicolon, Expected::SemicolonAfterCondition)?;

        // Compile the update expression, if any
        if !self.check(TokenKind::RightParen) {
            // Need to jump over the update expression after running the condition
            let update_jump = self.emit_jump(OpCode::Jump);
            let update_start = self.chunk().code.len();

            self.expression()?;
            // We also have to discard its value
            self.emit_opcode(OpCode::Pop);

            // Also need to jump back to condition
            self.emit_loop(loop_start)?;
            loop_start = update_start;

            self.patch_jump(update_jump)?;
        }

        self.consume(TokenKind::RightParen, Expected::RightParen)?;

        self.begin_loop(loop_start);

        // Compile the body
        self.statement()?;
        // Append a jump back to the start of the loop
        self.emit_loop(loop_start)?;

        // Ok, the loop body is done, now patch the exit jump if present
        if exit_jump != -1 {
            self.patch_jump(exit_jump as usize)?;
            // Also pop the condition
            self.emit_opcode(OpCode::Pop);
        }

        self.end_loop()?;
        self.end_scope();
        Ok(())
    }

    fn return_stmt(&mut self) -> Result<'a, ()> {
        if self.contexts.is_empty() {
            return Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::ReturnOutsideFunction,
            ));
        }

        if self.check(TokenKind::Semicolon) {
            // Implicitly return `this` in initializers and `nil` otherwise
            self.emit_return()?;
        } else {
            if self.is_initializer {
                return Err(CompileError::new(
                    self.prev_token.clone(),
                    CompileErrorKind::ReturnInInitializer,
                ));
            }
            self.expression()?;
        }

        self.emit_opcode(OpCode::Return);
        self.consume(TokenKind::Semicolon, Expected::Semicolon)
    }

    fn expression_statement(&mut self) -> Result<'a, ()> {
        self.expression()?;
        self.consume(TokenKind::Semicolon, Expected::Semicolon)?;
        self.emit_opcode(OpCode::Pop);

        Ok(())
    }

    fn expression(&mut self) -> Result<'a, ()> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn call(&mut self, _: bool) -> Result<'a, ()> {
        let arg_count = self.argument_list()?;

        self.emit_opcode(OpCode::Call);
        self.emit_byte(arg_count);

        Ok(())
    }

    fn argument_list(&mut self) -> Result<'a, u8> {
        const MAX_PARAMS: u8 = 255;
        let mut arity: u8 = 0;

        if !self.check(TokenKind::RightParen) {
            loop {
                if arity == MAX_PARAMS {
                    return Err(CompileError::new(
                        self.curr_token.clone(),
                        CompileErrorKind::TooManyArguments,
                    ));
                }

                arity += 1;
                self.expression()?;

                if !self.check(TokenKind::Comma) {
                    break;
                }

                // Consume the comma
                self.advance()?;
            }
        }

        // Consume the closing parenthesis
        self.consume(TokenKind::RightParen, Expected::RightParenAfterArguments)?;

        Ok(arity)
    }

    fn variable(&mut self, can_assign: bool) -> Result<'a, ()> {
        let name = self.prev_token.lexeme;
        let index = Self::resolve_local(&self.locals, name);

        // Pick local or global ops and final index
        let (get_op, get_op_long, set_op, set_op_long, idx) = if index != -1 {
            if !self.locals[index as usize].initialized {
                return Err(CompileError::new(
                    self.prev_token.to_owned(),
                    CompileErrorKind::VariableInOwnInitializer(name.to_string()),
                ));
            }

            (
                OpCode::GetLocal,
                OpCode::GetLocalLong,
                OpCode::SetLocal,
                OpCode::SetLocalLong,
                index as usize,
            )
        } else {
            let index = self.resolve_upvalue(name);

            if index != -1 {
                (
                    OpCode::GetUpvalue,
                    OpCode::GetUpvalueLong,
                    OpCode::SetUpvalue,
                    OpCode::SetUpvalueLong,
                    index as usize,
                )
            } else {
                (
                    OpCode::GetGlobal,
                    OpCode::GetGlobalLong,
                    OpCode::SetGlobal,
                    OpCode::SetGlobalLong,
                    self.sym_table.resolve(name),
                )
            }
        };

        // Assignment or read
        if can_assign && self.curr_token.kind == TokenKind::Equal {
            self.advance()?;
            self.expression()?;
            self.emit_opcode_with_num(set_op, set_op_long, idx, CompileErrorKind::TooManyGlobals)
        } else {
            self.emit_opcode_with_num(get_op, get_op_long, idx, CompileErrorKind::TooManyGlobals)
        }
    }

    fn number(&mut self, _: bool) -> Result<'a, ()> {
        match self.prev_token.lexeme.parse::<f64>() {
            Ok(value) => self.emit_opcode_with_constant_long(
                OpCode::Constant,
                OpCode::ConstantLong,
                Value::Number(value),
            ),
            Err(_) => Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::InvalidNumber,
            )),
        }
    }

    fn literal(&mut self, _: bool) -> Result<'a, ()> {
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
            _ => unreachable!("literal() called on a non-literal token"),
        }
    }

    fn string(&mut self, _: bool) -> Result<'a, ()> {
        match self.prev_token.kind {
            TokenKind::String => {
                let s = &self.prev_token.lexeme[1..self.prev_token.lexeme.len() - 1];
                let str_ptr = self.str_intern_table.intern_slice(s, self.gc);

                self.emit_opcode_with_constant_long(
                    OpCode::Constant,
                    OpCode::ConstantLong,
                    Value::String(str_ptr),
                )
            }
            _ => unreachable!("string() called on a non-string token"),
        }
    }

    fn grouping(&mut self, _: bool) -> Result<'a, ()> {
        self.expression()?;
        self.consume(TokenKind::RightParen, Expected::RightParen)
    }

    fn unary(&mut self, _: bool) -> Result<'a, ()> {
        let operator_kind = self.prev_token.kind;
        let operator_line = self.prev_token.line;

        // Compile the operand
        self.parse_precedence(Precedence::Unary)?;

        // Emit the operator instruction at the operator's line, not the
        // operand's, so errors point at the operator even across line breaks.
        match operator_kind {
            TokenKind::Minus => self.emit_opcode_at_line(OpCode::Negate, operator_line),
            TokenKind::Bang => self.emit_opcode_at_line(OpCode::Not, operator_line),
            _ => unreachable!("unary() called on a non-unary operator"),
        }

        Ok(())
    }

    fn binary(&mut self, _: bool) -> Result<'a, ()> {
        let operator_token = self.prev_token.clone();
        let operator_kind = self.prev_token.kind;
        let operator_line = operator_token.line;

        // Compile the operand
        self.parse_precedence(self.get_rule(operator_kind).precedence + 1)?;

        // Emit the operator instruction at the operator's line, not the
        // operand's, so errors point at the operator even across line breaks.
        match operator_kind {
            TokenKind::Plus => self.emit_opcode_at_line(OpCode::Add, operator_line),
            TokenKind::Minus => self.emit_opcode_at_line(OpCode::Sub, operator_line),
            TokenKind::Star => self.emit_opcode_at_line(OpCode::Mult, operator_line),
            TokenKind::Slash => self.emit_opcode_at_line(OpCode::Divide, operator_line),
            TokenKind::EqualEqual => self.emit_opcode_at_line(OpCode::Equal, operator_line),
            TokenKind::BangEqual => self.emit_opcode_at_line(OpCode::NotEqual, operator_line),
            TokenKind::Greater => self.emit_opcode_at_line(OpCode::Greater, operator_line),
            TokenKind::GreaterEqual => {
                self.emit_opcode_at_line(OpCode::GreaterEqual, operator_line)
            }
            TokenKind::Less => self.emit_opcode_at_line(OpCode::Less, operator_line),
            TokenKind::LessEqual => self.emit_opcode_at_line(OpCode::LessEqual, operator_line),
            _ => unreachable!("binary() called on a non-binary operator"),
        }

        Ok(())
    }

    fn ternary(&mut self, _: bool) -> Result<'a, ()> {
        let operator_kind = self.prev_token.kind;

        if let TokenKind::Question = operator_kind {
            // Compile the 2nd operand
            self.parse_precedence(Precedence::Assignment)?;

            // Consume the colon
            self.consume(TokenKind::Colon, Expected::Colon)?;

            // Compile the 3rd operand
            self.parse_precedence(Precedence::Assignment)?;

            self.emit_opcode(OpCode::Ternary);
            Ok(())
        } else {
            unreachable!("ternary() called on a non-'?' operator")
        }
    }

    fn logical_or(&mut self, _: bool) -> Result<'a, ()> {
        let operator_kind = self.prev_token.kind;

        if let TokenKind::Or = operator_kind {
            let then_jump = self.emit_jump(OpCode::JumpIfTrue);

            // We'll just flow-through to the next instruction if the left operand is false
            // Compile the right operand
            self.emit_opcode(OpCode::Pop);
            self.parse_precedence(Precedence::And)?;
            self.patch_jump(then_jump)?;

            Ok(())
        } else {
            unreachable!("logical_or() called on a non-'or' operator")
        }
    }

    fn logical_and(&mut self, _: bool) -> Result<'a, ()> {
        let operator_kind = self.prev_token.kind;

        if let TokenKind::And = operator_kind {
            let then_jump = self.emit_jump(OpCode::JumpIfFalse);

            // We'll just flow-through to the next instruction if the left operand is true
            // Compile the right operand
            self.emit_opcode(OpCode::Pop);
            self.parse_precedence(Precedence::Equality)?;
            self.patch_jump(then_jump)?;

            Ok(())
        } else {
            unreachable!("logical_and() called on a non-'and' operator")
        }
    }

    fn continue_stmt(&mut self) -> Result<'a, ()> {
        let (loop_start, scope_depth) = if let Some(loop_context) = self.innermost_loop() {
            (loop_context.loop_start, loop_context.scope_depth)
        } else {
            return Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::ContinueOutsideLoop,
            ));
        };

        self.consume(TokenKind::Semicolon, Expected::Semicolon)?;

        // Pop the locals in the loop body
        self.emit_pop_scopes(scope_depth);

        // Jump back to the start of the loop
        self.emit_loop(loop_start)
    }

    fn break_stmt(&mut self) -> Result<'a, ()> {
        let scope_depth = if let Some(loop_context) = self.innermost_loop() {
            loop_context.scope_depth
        } else {
            return Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::BreakOutsideLoop,
            ));
        };

        self.consume(TokenKind::Semicolon, Expected::Semicolon)?;

        // Pop the locals in the loop body
        self.emit_pop_scopes(scope_depth);

        // Emit a jump to the end of the loop
        let break_jump = self.emit_jump(OpCode::Jump);

        // Push the jump to the loop context
        self.loop_contexts
            .last_mut()
            .unwrap()
            .break_jumps
            .push(break_jump);
        Ok(())
    }

    fn dot(&mut self, can_assign: bool) -> Result<'a, ()> {
        self.consume(TokenKind::Identifier, Expected::PropertyName)?;

        let name = self.prev_token.lexeme;
        let name_ptr = self.str_intern_table.intern_slice(name, self.gc);

        if can_assign && self.check(TokenKind::Equal) {
            self.advance()?;
            self.expression()?;
            self.emit_opcode_with_constant(OpCode::SetProperty, Value::String(name_ptr))
        } else if self.check(TokenKind::LeftParen) {
            // Immediate method invocation
            self.advance()?;

            let arg_count = self.argument_list()?;
            self.emit_opcode_with_constant(OpCode::Invoke, Value::String(name_ptr))?;
            self.emit_byte(arg_count);
            Ok(())
        } else {
            self.emit_opcode_with_constant(OpCode::GetProperty, Value::String(name_ptr))
        }
    }

    fn this(&mut self, _: bool) -> Result<'a, ()> {
        if self.class_contexts.is_empty() {
            return Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::ThisOutsideClass,
            ));
        }

        self.variable(false)
    }

    fn super_(&mut self, _: bool) -> Result<'a, ()> {
        if self.class_contexts.is_empty() {
            return Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::SuperOutsideClass,
            ));
        }

        if !self.class_contexts.last().unwrap().has_superclass {
            return Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::SuperWithoutSuperclass,
            ));
        }

        self.consume(TokenKind::Dot, Expected::DotAfterSuper)?;
        self.consume(TokenKind::Identifier, Expected::SuperclassMethodName)?;

        let name = self.prev_token.lexeme;
        let name_ptr = self.str_intern_table.intern_slice(name, self.gc);

        // Load `this` and `super`
        self.resolve_variable("this")?;

        if self.check(TokenKind::LeftParen) {
            // Immediate method invocation
            self.advance()?;

            let arg_count = self.argument_list()?;
            self.resolve_variable("super")?;
            self.emit_opcode_with_constant(OpCode::SuperInvoke, Value::String(name_ptr))?;
            self.emit_byte(arg_count);
            Ok(())
        } else {
            self.resolve_variable("super")?;
            self.emit_opcode_with_constant(OpCode::GetSuper, Value::String(name_ptr))
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<'a, ()> {
        self.advance()?;

        let prefix_rule = self.get_rule(self.prev_token.kind).prefix_rule;
        let can_assign = precedence <= Precedence::Assignment;

        match prefix_rule {
            Some(prefix_rule) => prefix_rule(self, can_assign)?,
            None => {
                return Err(CompileError::new(
                    self.prev_token.clone(),
                    CompileErrorKind::Expected(Expected::Expression),
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
                        CompileErrorKind::Expected(Expected::Expression),
                    ))
                }
            }
        }

        if can_assign && self.check(TokenKind::Equal) {
            Err(CompileError::new(
                self.curr_token.clone(),
                CompileErrorKind::InvalidAssignmentTarget,
            ))
        } else {
            Ok(())
        }
    }

    fn advance(&mut self) -> Result<'a, ()> {
        let token = self.scanner.scan_token();

        self.prev_token = self.curr_token.clone();
        self.curr_token = token.clone();

        if let TokenKind::Error = token.kind {
            let scan_error = self
                .scanner
                .take_error()
                .expect("scanner produced an error token without a recorded error");
            return Err(CompileError::new(token, CompileErrorKind::Scan(scan_error)));
        }

        Ok(())
    }

    fn consume(&mut self, expected: TokenKind, missing: Expected) -> Result<'a, ()> {
        if self.check(expected) {
            self.advance()
        } else {
            Err(CompileError::new(
                self.curr_token.clone(),
                CompileErrorKind::Expected(missing),
            ))
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

    fn finish(mut self) -> Option<Function> {
        // `emit_return` will emit a `nil` since `finish` is only called from the global scope
        let _err = self.emit_return();

        if !self.had_error {
            Some(self.function)
        } else {
            None
        }
    }

    /// Declare local with `initialized` set to false
    fn declare_local(&mut self, name: &'a str) -> Result<'a, usize> {
        for local in self.locals.iter().rev() {
            if local.depth < self.curr_depth {
                break;
            }

            if local.name == name {
                return Err(CompileError::new(
                    self.prev_token.to_owned(),
                    CompileErrorKind::RedeclaredVariable(name.to_string()),
                ));
            }
        }

        self.locals
            .push(Local::new(name, self.curr_depth, false, false));

        Ok(self.locals.len() - 1)
    }

    /// Declares a variable with the given name in the current scope. This will also mark the local
    /// variable as initialized
    fn declare_variable(&mut self, name: &'a str) -> Result<'a, usize> {
        let index = if self.curr_depth == 0 {
            // Global variable
            self.sym_table.declare(name)
        } else {
            // Local variable
            let index = self.declare_local(name)?;
            self.mark_as_initialized(index);
            index
        };

        Ok(index)
    }

    /// Mark the local as being initialized
    fn mark_as_initialized(&mut self, index: usize) {
        self.locals[index].initialized = true;
    }

    /// Returns the index of the first declaration of the
    /// given local in the given slice, -1 otherwise
    fn resolve_local(locals: &[Local], name: &'a str) -> i32 {
        for (index, local) in locals.iter().rev().enumerate() {
            if local.name == name {
                return (locals.len() - index - 1) as i32;
            }
        }

        -1
    }

    /// Resolves the given name in the chain of function scopes starting from the current
    /// function upto the global scope and returns the index of the upvalue if found, -1 otherwise
    fn resolve_upvalue(&mut self, name: &'a str) -> i32 {
        // The current context is not stored in `self.contexts` so we've to handle it separately
        let len = self.contexts.len();

        if len == 0 {
            // We are at the global scope
            return -1;
        }

        // Check if the name is a local variable in the scope of the enclosing function
        let index = Self::resolve_local(&self.contexts.last().unwrap().locals, name);

        if index != -1 {
            // The name is a local variable in the enclosing function. Mark it as captured
            let index = index as usize;
            self.contexts.last_mut().unwrap().locals[index].captured = true;

            return Self::add_upvalue(&mut self.upvalues, true, index) as i32;
        }

        // Check if the name is an upvalue in the enclosing function
        let index = Self::resolve_upvalue_helper(&mut self.contexts, name);

        if index != -1 {
            // The name is an upvalue in the enclosing function
            Self::add_upvalue(&mut self.upvalues, false, index as usize) as i32
        } else {
            -1
        }
    }

    /// Resolves the given name in the chain of function scopes starting from the current
    /// function upto the global scope and returns the index of the upvalue if found, -1 otherwise
    fn resolve_upvalue_helper(contexts: &mut [CompilerContext], name: &'a str) -> i32 {
        // If there is only one context, we've reached the global scope,
        // so the name must be a global variable (or it is undefined)
        let len = contexts.len();

        if len == 1 {
            // We are at the global scope
            return -1;
        }

        // Check if the name is a local variable in the scope of the enclosing function
        let index = Self::resolve_local(&contexts[len - 2].locals, name);

        if index != -1 {
            // The name is a local variable in the enclosing function. Mark it as captured
            let index = index as usize;
            contexts[len - 2].locals[index].captured = true;

            return Self::add_upvalue(&mut contexts.last_mut().unwrap().upvalues, true, index)
                as i32;
        }

        // Check if the name is an upvalue in the enclosing function
        let index = Self::resolve_upvalue_helper(&mut contexts[..len - 1], name);

        if index != -1 {
            // The name is an upvalue in the enclosing function
            Self::add_upvalue(
                &mut contexts.last_mut().unwrap().upvalues,
                false,
                index as usize,
            ) as i32
        } else {
            -1
        }
    }

    /// Resolves the given variable
    fn resolve_variable(&mut self, name: &'a str) -> Result<'a, ()> {
        let index = Self::resolve_local(&self.locals, name);

        // Pick local or global ops and final index
        let (get_op, get_op_long, idx) = if index != -1 {
            if !self.locals[index as usize].initialized {
                return Err(CompileError::new(
                    self.prev_token.to_owned(),
                    CompileErrorKind::VariableInOwnInitializer(name.to_string()),
                ));
            }

            (OpCode::GetLocal, OpCode::GetLocalLong, index as usize)
        } else {
            let index = self.resolve_upvalue(name);

            if index != -1 {
                (OpCode::GetUpvalue, OpCode::GetUpvalueLong, index as usize)
            } else {
                (
                    OpCode::GetGlobal,
                    OpCode::GetGlobalLong,
                    self.sym_table.resolve(name),
                )
            }
        };

        self.emit_opcode_with_num(get_op, get_op_long, idx, CompileErrorKind::TooManyGlobals)
    }

    /// Adds the a new upvalue to the current function
    fn add_upvalue(dest: &mut Vec<Upvalue>, is_local: bool, index: usize) -> usize {
        // Check if the upvalue already exists
        for (i, upvalue) in dest.iter().enumerate() {
            if upvalue.is_local == is_local && upvalue.index == index {
                return i;
            }
        }

        // Add a new upvalue
        dest.push(Upvalue { is_local, index });
        dest.len() - 1
    }

    /// Increases the current scope depth
    fn begin_scope(&mut self) {
        self.curr_depth += 1;
    }

    /// Decreases the scope depth to the one below the given depth and
    /// pops all locals upto (including) that depth. Also emits instructions
    /// to close-over the locals that have been captured by upvalues
    fn end_scope(&mut self) {
        while let Some(local) = self.locals.last() {
            if local.depth < self.curr_depth {
                break;
            }

            if local.captured {
                self.emit_opcode(OpCode::CloseUpvalue);
            } else {
                self.emit_opcode(OpCode::Pop);
            }

            self.locals.pop();
        }

        self.curr_depth -= 1;
    }

    /// Emits instructions to pop (or close-over) all locals upto (but excluding) the given depth
    fn emit_pop_scopes(&mut self, upto_depth: usize) {
        let mut chunk = std::mem::take(&mut self.function.chunk);
        let rev_iter = self.locals.iter().rev();

        for local in rev_iter {
            if local.depth <= upto_depth {
                break;
            }

            if local.captured {
                chunk.write_opcode(OpCode::CloseUpvalue, self.prev_token.line);
            } else {
                chunk.write_opcode(OpCode::Pop, self.prev_token.line);
            }
        }

        self.function.chunk = chunk;
    }

    /// Pushes a new loop context
    fn begin_loop(&mut self, loop_start: usize) {
        self.loop_contexts.push(LoopContext {
            loop_start,
            scope_depth: self.curr_depth,
            break_jumps: Vec::new(),
        });
    }

    /// Pops the topmost loop context
    fn end_loop(&mut self) -> Result<'a, ()> {
        // Patch all the break statements in the loop body
        let break_jumps = self.loop_contexts.pop().unwrap().break_jumps;

        for jump_offset in break_jumps {
            self.patch_jump(jump_offset)?;
        }

        Ok(())
    }

    /// Returns the topmost (innermost) loop context
    fn innermost_loop(&self) -> Option<&LoopContext> {
        self.loop_contexts.last()
    }

    /// Saves the current compilation context and sets up a new one for the given function
    fn push_context(&mut self, func_name: &str, is_initializer: bool) {
        // Save current context
        let saved_context = CompilerContext {
            function: std::mem::replace(
                &mut self.function,
                Function {
                    name: func_name.to_owned(),
                    arity: 0,
                    upvalue_count: 0,
                    chunk: Chunk::new(),
                },
            ),
            locals: std::mem::take(&mut self.locals),
            curr_depth: std::mem::take(&mut self.curr_depth),
            loop_contexts: std::mem::take(&mut self.loop_contexts),
            upvalues: std::mem::take(&mut self.upvalues),
            is_initializer: std::mem::replace(&mut self.is_initializer, is_initializer),
        };

        self.contexts.push(saved_context);
    }

    /// Restores the previous compilation context and returns the compiled function
    fn pop_context(&mut self) -> Function {
        let compiled_function = std::mem::take(&mut self.function);

        // There will always be a saved context
        let saved_context = self.contexts.pop().unwrap();

        self.function = saved_context.function;
        self.locals = saved_context.locals;
        self.curr_depth = saved_context.curr_depth;
        self.loop_contexts = saved_context.loop_contexts;
        self.upvalues = saved_context.upvalues;
        self.is_initializer = saved_context.is_initializer;

        compiled_function
    }

    fn chunk(&mut self) -> &mut Chunk {
        &mut self.function.chunk
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.prev_token.line;

        self.chunk().write_byte(byte, line);
    }

    fn emit_opcode(&mut self, opcode: OpCode) {
        let line = self.prev_token.line;

        self.chunk().write_opcode(opcode, line);
    }

    fn emit_opcode_at_line(&mut self, opcode: OpCode, line: usize) {
        self.chunk().write_opcode(opcode, line);
    }

    fn emit_return(&mut self) -> Result<'a, ()> {
        // Implicitly return `this` in initializers and `nil` otherwise
        if self.is_initializer {
            self.emit_opcode_with_num(
                OpCode::GetLocal,
                OpCode::GetLocalLong,
                0,
                CompileErrorKind::TooManyLocals,
            )?;
        } else {
            self.emit_opcode(OpCode::Nil);
        }

        self.emit_opcode(OpCode::Return);
        Ok(())
    }

    fn emit_opcode_with_num(
        &mut self,
        opcode: OpCode,
        opcode_long: OpCode,
        num: usize,
        err: CompileErrorKind,
    ) -> Result<'a, ()> {
        const MAX24BIT: usize = (1 << 24) - 1;

        if num <= u8::MAX as usize {
            self.emit_opcode(opcode);
            self.emit_byte(num as u8);
            Ok(())
        } else if num <= MAX24BIT {
            let line = self.prev_token.line;

            self.emit_opcode(opcode_long);
            self.chunk().write_int24(num, line);
            Ok(())
        } else {
            Err(CompileError::new(self.prev_token.clone(), err))
        }
    }

    fn emit_opcode_with_constant(&mut self, opcode: OpCode, value: Value) -> Result<'a, ()> {
        let index = self.chunk().add_constant(value);

        if index <= u8::MAX as usize {
            self.emit_opcode(opcode);
            self.emit_byte(index as u8);
            Ok(())
        } else {
            Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::TooManyConstants,
            ))
        }
    }

    fn emit_opcode_with_constant_long(
        &mut self,
        opcode: OpCode,
        opcode_long: OpCode,
        value: Value,
    ) -> Result<'a, ()> {
        let index = self.chunk().add_constant(value);
        self.emit_opcode_with_num(
            opcode,
            opcode_long,
            index,
            CompileErrorKind::TooManyConstants,
        )
    }

    /// Emits a jump instruction and returns the location of the first byte of the jump address
    fn emit_jump(&mut self, opcode: OpCode) -> usize {
        let line = self.prev_token.line;

        self.chunk().write_opcode(opcode, line);
        self.chunk().write_bytes(&[0; 2], &[line; 2]);
        self.chunk().code.len() - 2
    }

    fn patch_jump(&mut self, offset: usize) -> Result<'a, ()> {
        const BYTE_MASK: usize = (1usize << 8) - 1;

        let jump_dist = self.chunk().code.len() - offset - 2; // -2 for the operands

        if jump_dist > u16::MAX as usize {
            Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::JumpTooLarge,
            ))
        } else {
            self.chunk().code[offset] = ((jump_dist >> 8) & BYTE_MASK) as u8;
            self.chunk().code[offset + 1] = (jump_dist & BYTE_MASK) as u8;
            Ok(())
        }
    }

    fn emit_loop(&mut self, loop_start: usize) -> Result<'a, ()> {
        // Jumps to the start of the loop
        const BYTE_MASK: usize = (1usize << 8) - 1;

        self.emit_opcode(OpCode::Loop);

        let jump_dist = self.chunk().code.len() - loop_start + 2; // +2 for the operands

        if jump_dist > u16::MAX as usize {
            Err(CompileError::new(
                self.prev_token.clone(),
                CompileErrorKind::JumpTooLarge,
            ))
        } else {
            self.emit_byte(((jump_dist >> 8) & BYTE_MASK) as u8);
            self.emit_byte((jump_dist & BYTE_MASK) as u8);
            Ok(())
        }
    }

    fn get_rule(&self, kind: TokenKind) -> &ParseRule<'a, 'b, W> {
        &Self::RULES[kind.as_usize()]
    }

    fn report_err(&mut self, err: CompileError<'a>) {
        self.had_error = true;

        let mut rendered = String::new();
        err.render(self.source, &mut rendered);
        // Blank line separates consecutive diagnostics.
        writeln!(self.err_stream, "{rendered}").unwrap();
    }
}
