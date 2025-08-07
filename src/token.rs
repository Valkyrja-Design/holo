#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // parenthesis
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,

    Semicolon,
    Question,
    Colon,

    // operators
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

    // literals
    Identifier,
    String,
    Number,

    // keywords,
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
