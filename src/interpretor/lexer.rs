use crate::prelude::*;
use logos::Logos;

#[derive(IsVariant, Logos, Debug, Clone, PartialEq)]
pub enum Token {
    #[token("..")]
    Join,
    #[token("|")]
    Pipe,
    #[token(".")]
    Deref,
    #[token(";")]
    Comma,
    #[token(",")]
    Separator,
    #[token("=")]
    Assign,
    #[token("=>")]
    Fn,
    #[token("(")]
    GroupOpen,
    #[token(")")]
    GroupClose,
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
    #[regex("[a-zA-Z_]+[a-zA-Z_0-9-]*")]
    Ident,
    #[regex("-?[0-9]+")]
    LiteralInteger,
    #[regex("[0-9]*\\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+")]
    LiteralFloat,
    #[regex(r#""([^"\\]|\\t|\\u|\\n|\\")*""#)]
    LiteralString,
    #[regex("let")]
    Keyword,
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl Token {
    pub fn is_literal(&self) -> bool {
        match self {
            Token::LiteralString | Token::LiteralInteger | Token::LiteralFloat => true,
            _ => false,
        }
    }
}