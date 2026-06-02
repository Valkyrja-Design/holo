//! Structured error types and diagnostic rendering for the Holo programming language.
//!
//! Errors are split into three categories according to the subsystem reporting them:
//! - [`ScanError`] for lexical errors produced by the scanner,
//! - [`CompileError`] for errors produced while parsing/compiling, rendered as
//!   rustc-style diagnostics with a source snippet,
//! - [`RuntimeError`] for errors raised by the virtual machine at run time.

use std::fmt::{self, Display, Write as _};

use crate::token::{Token, TokenKind};

/// A lexical error detected by the scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError {
    /// A character that cannot start any token.
    UnexpectedChar(char),
    /// A string literal that was never closed before end of file.
    UnterminatedString,
    /// A block comment that was never closed before end of file.
    UnterminatedComment,
}

impl Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanError::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
            ScanError::UnterminatedString => write!(f, "unterminated string literal"),
            ScanError::UnterminatedComment => write!(f, "unterminated block comment"),
        }
    }
}

/// The grammatical element the parser expected but did not find.
///
/// Used by [`CompileError::Expected`] to describe what the parser was looking
/// for when it encountered an unexpected token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expected {
    Expression,
    VariableName,
    FunctionName,
    ClassName,
    SuperclassName,
    MethodName,
    ParameterName,
    PropertyName,
    SuperclassMethodName,
    Semicolon,
    SemicolonAfterCondition,
    LeftParen,
    LeftParenAfterFunctionName,
    RightParen,
    RightParenAfterParameters,
    RightParenAfterArguments,
    LeftBraceBeforeFunctionBody,
    LeftBraceBeforeClassBody,
    RightBraceAfterClassBody,
    RightBraceToCloseBlock,
    RightBraceToCloseInterpolation,
    Colon,
    DotAfterSuper,
}

impl Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Expected::Expression => "expression",
            Expected::VariableName => "variable name",
            Expected::FunctionName => "function name",
            Expected::ClassName => "class name",
            Expected::SuperclassName => "superclass name after ':'",
            Expected::MethodName => "method name",
            Expected::ParameterName => "parameter name",
            Expected::PropertyName => "property name after '.'",
            Expected::SuperclassMethodName => "superclass method name",
            Expected::Semicolon => "';' after statement",
            Expected::SemicolonAfterCondition => "';' after loop condition",
            Expected::LeftParen => "'('",
            Expected::LeftParenAfterFunctionName => "'(' after function name",
            Expected::RightParen => "')'",
            Expected::RightParenAfterParameters => "')' after parameters",
            Expected::RightParenAfterArguments => "')' after arguments",
            Expected::LeftBraceBeforeFunctionBody => "'{' before function body",
            Expected::LeftBraceBeforeClassBody => "'{' before class body",
            Expected::RightBraceAfterClassBody => "'}' after class body",
            Expected::RightBraceToCloseBlock => "'}' to close block",
            Expected::RightBraceToCloseInterpolation => "'}' to close interpolation",
            Expected::Colon => "':'",
            Expected::DotAfterSuper => "'.' after 'super'",
        };
        f.write_str(s)
    }
}

/// The kind of error produced while compiling source into bytecode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileErrorKind {
    /// A lexical error surfaced from the scanner.
    Scan(ScanError),
    /// The parser expected a particular grammatical element.
    Expected(Expected),
    /// An assignment target that is not an l-value.
    InvalidAssignmentTarget,
    /// A numeric literal that could not be parsed as an `f64`.
    InvalidNumber,
    /// A variable referenced inside its own initializer.
    VariableInOwnInitializer(String),
    /// A variable declared twice in the same scope.
    RedeclaredVariable(String),
    /// A class listed itself as its own superclass.
    InheritFromSelf,
    /// A `return` statement outside of any function body.
    ReturnOutsideFunction,
    /// A value returned from a class initializer.
    ReturnInInitializer,
    /// A `continue` statement outside of any loop.
    ContinueOutsideLoop,
    /// A `break` statement outside of any loop.
    BreakOutsideLoop,
    /// `this` used outside of a method.
    ThisOutsideClass,
    /// `super` used outside of a class.
    SuperOutsideClass,
    /// `super` used in a class that has no superclass.
    SuperWithoutSuperclass,
    /// More than 255 parameters in a function declaration.
    TooManyParameters,
    /// More than 255 arguments in a call expression.
    TooManyArguments,
    /// More upvalues captured by a closure than the bytecode can encode.
    TooManyUpvalues,
    /// More constants in a chunk than the bytecode can encode.
    TooManyConstants,
    /// More global variables than the bytecode can encode.
    TooManyGlobals,
    /// More local variables than the bytecode can encode.
    TooManyLocals,
    /// A jump offset too large to encode.
    JumpTooLarge,
}

impl Display for CompileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileErrorKind::Scan(err) => write!(f, "{err}"),
            CompileErrorKind::Expected(expected) => write!(f, "expected {expected}"),
            CompileErrorKind::InvalidAssignmentTarget => f.write_str("invalid assignment target"),
            CompileErrorKind::InvalidNumber => f.write_str("invalid number literal"),
            CompileErrorKind::VariableInOwnInitializer(name) => {
                write!(f, "cannot read variable '{name}' in its own initializer")
            }
            CompileErrorKind::RedeclaredVariable(name) => {
                write!(f, "variable '{name}' is already declared in this scope")
            }
            CompileErrorKind::InheritFromSelf => f.write_str("a class cannot inherit from itself"),
            CompileErrorKind::ReturnOutsideFunction => {
                f.write_str("'return' can only be used inside a function")
            }
            CompileErrorKind::ReturnInInitializer => {
                f.write_str("cannot return a value from an initializer")
            }
            CompileErrorKind::ContinueOutsideLoop => {
                f.write_str("'continue' can only be used inside a loop")
            }
            CompileErrorKind::BreakOutsideLoop => {
                f.write_str("'break' can only be used inside a loop")
            }
            CompileErrorKind::ThisOutsideClass => {
                f.write_str("'this' can only be used inside a method")
            }
            CompileErrorKind::SuperOutsideClass => {
                f.write_str("'super' can only be used inside a class")
            }
            CompileErrorKind::SuperWithoutSuperclass => {
                f.write_str("'super' can only be used in a class with a superclass")
            }
            CompileErrorKind::TooManyParameters => {
                f.write_str("cannot have more than 255 parameters")
            }
            CompileErrorKind::TooManyArguments => {
                f.write_str("cannot have more than 255 arguments")
            }
            CompileErrorKind::TooManyUpvalues => {
                f.write_str("too many variables captured by a closure")
            }
            CompileErrorKind::TooManyConstants => f.write_str("too many constants in one chunk"),
            CompileErrorKind::TooManyGlobals => f.write_str("too many global variables"),
            CompileErrorKind::TooManyLocals => f.write_str("too many local variables"),
            CompileErrorKind::JumpTooLarge => f.write_str("jump distance is too large to encode"),
        }
    }
}

/// A compile error together with the source location it refers to.
pub struct CompileError<'a> {
    pub kind: CompileErrorKind,
    pub token: Token<'a>,
}

impl<'a> CompileError<'a> {
    pub fn new(token: Token<'a>, kind: CompileErrorKind) -> Self {
        CompileError { kind, token }
    }

    /// Renders this error as a rustc-style diagnostic into `out`.
    ///
    /// `source` must be the full source text the token was scanned from so the
    /// offending line can be shown with a caret underneath the token.
    pub fn render(&self, source: &str, out: &mut String) {
        let line_no = self.token.line.max(1);
        let src_line = source.lines().nth(line_no - 1).unwrap_or("");
        let line_width = src_line.chars().count();

        // Determine where the caret points and how wide it is.
        let (caret_col, caret_len) = match self.token.kind {
            // Past the end of input: point just after the last character.
            TokenKind::Eof => (line_width + 1, 1),
            // Unterminated literals span to end of file, so a single caret at
            // the opening delimiter reads more clearly than a giant underline.
            _ => match &self.kind {
                CompileErrorKind::Scan(
                    ScanError::UnterminatedString | ScanError::UnterminatedComment,
                ) => (self.token.column.max(1), 1),
                _ => (
                    self.token.column.max(1),
                    self.token.lexeme.chars().count().max(1),
                ),
            },
        };

        let line_str = line_no.to_string();
        let gutter = " ".repeat(line_str.len());
        let pad = " ".repeat(caret_col - 1);
        let carets = "^".repeat(caret_len);

        // `write!` into a String is infallible; the `?`-free `.ok()` keeps the
        // call sites tidy without an unwrap.
        let _ = writeln!(out, "error: {}", self.kind);
        let _ = writeln!(out, "{gutter}--> line {line_no}:{caret_col}");
        let _ = writeln!(out, "{gutter} |");
        let _ = writeln!(out, "{line_str} | {src_line}");
        let _ = writeln!(out, "{gutter} | {pad}{carets}");
    }
}

/// An error raised by the virtual machine while executing bytecode.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// Operand to unary `-` was not a number.
    NegateOperandNotNumber,
    /// Operand to unary `!` was not a bool.
    NotOperandNotBool,
    /// The predicate of a ternary expression was not a bool.
    TernaryPredicateNotBool,
    /// A loop or `if` condition was not a bool.
    ConditionNotBool,
    /// Both operands to a numeric binary operator must be numbers.
    /// Holds the operator lexeme (e.g. `"-"`, `">="`).
    BinaryOperandsNotNumbers(&'static str),
    /// Operands to `+` must be two numbers or two strings.
    AddOperandsInvalid,
    /// A property was accessed on a value that is not a class instance.
    PropertyOnNonInstance,
    /// A method was invoked on a value that is not a class instance.
    MethodOnNonInstance,
    /// A superclass expression did not evaluate to a class.
    SuperclassNotClass,
    /// A value that is not callable was called.
    NotCallable,
    /// A call passed the wrong number of arguments.
    ArgCountMismatch { expected: u8, got: u8 },
    /// A class initializer received arguments but takes none.
    InitializerArgCount(u8),
    /// A reference to an undefined global variable.
    UndefinedVariable(String),
    /// A reference to a method that does not exist on a class.
    UndefinedMethod(String),
    /// A reference to a property that does not exist on an instance.
    UndefinedProperty(String),
    /// The value stack exceeded its maximum size. Holds the limit.
    StackOverflow(usize),
    /// An error returned by a native function.
    Native(String),
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::NegateOperandNotNumber => f.write_str("Operand to '-' must be a number"),
            RuntimeError::NotOperandNotBool => f.write_str("Operand to '!' must be a bool"),
            RuntimeError::TernaryPredicateNotBool => {
                f.write_str("Expected a boolean as ternary operator predicate")
            }
            RuntimeError::ConditionNotBool => f.write_str("Expected `bool` as condition"),
            RuntimeError::BinaryOperandsNotNumbers(op) => {
                write!(f, "Operands to '{op}' must be numbers")
            }
            RuntimeError::AddOperandsInvalid => {
                f.write_str("Operands to '+' must be two numbers or strings")
            }
            RuntimeError::PropertyOnNonInstance => {
                f.write_str("Property must be accessed on a class instance")
            }
            RuntimeError::MethodOnNonInstance => {
                f.write_str("Can only call methods on class instances")
            }
            RuntimeError::SuperclassNotClass => f.write_str("Superclass must be a class"),
            RuntimeError::NotCallable => f.write_str("Can only call functions and classes"),
            RuntimeError::ArgCountMismatch { expected, got } => {
                write!(
                    f,
                    "Incorrect number of arguments: expected {expected}, got {got}"
                )
            }
            RuntimeError::InitializerArgCount(got) => {
                write!(f, "Expected 0 arguments for class initializer, got {got}")
            }
            RuntimeError::UndefinedVariable(name) => write!(f, "Undefined variable '{name}'"),
            RuntimeError::UndefinedMethod(name) => write!(f, "Undefined method '{name}'"),
            RuntimeError::UndefinedProperty(name) => write!(f, "Undefined property '{name}'"),
            RuntimeError::StackOverflow(limit) => {
                write!(f, "Stack overflow: maximum stack size is {limit}")
            }
            RuntimeError::Native(msg) => f.write_str(msg),
        }
    }
}
