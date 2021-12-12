use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token {
    #[token("..")]
    Join,
    #[token("|")]
    Pipe,
    #[token("$")]
    Deref,
    #[token(";")]
    Separator,
    #[token("(")]
    GroupOpen,
    #[token(")")]
    GroupClose,
    #[regex("([a-zA-Z_-]+|/+|\\*+)+")]
    Ident,
    // #[regex("/\"(?:\\.)*\"/")]
    #[regex(r#"("([^"\\]|\\t|\\u|\\n|\\")*")|([a-zA-Z_-]*|[0-9]*[a-zA-Z_-]*)"#)]
    LiteralString,
    #[regex("-?[0-9]+")]
    LiteralInteger,
    #[regex("[0-9]*\\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+")]
    LiteralFloat,
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}
