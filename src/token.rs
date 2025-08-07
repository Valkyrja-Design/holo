#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // Parenthesis
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,

    Semicolon,
    Question,
    Colon,

    // Operators
    Comma,
    Dot,
    Minus,
    Plus,
    Slash,
    Star,
    Bang,
    BangEqual, // !, !=
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    PlusPlus,
    PlusEqual,
    MinusMinus,
    MinusEqual,
    StarEqual,
    SlashEqual,

    // Literals
    Identifier,
    String,
    Number,

    // Keywords,
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Break,
    Continue,

    Error,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub lexeme: &'a str,
    pub line: usize,
}

impl TokenKind {
    pub fn as_usize(self) -> usize {
        self as usize
    }
}
