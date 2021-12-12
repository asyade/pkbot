use super::Token;
use crate::prelude::*;
use logos::Lexer;

#[derive(Debug, Clone)]
pub enum CommandAstBody {
    Call { name: String },
    Arguments { arguments: Vec<String> },
    Pipe,
    Separator,
}

impl CommandAstBody {
    pub fn call_name(self) -> String {
        match self {
            CommandAstBody::Call { name } => name,
            _ => panic!("Expected call"),
        }
    }

    pub fn arguments(self) -> Vec<String> {
        match self {
            CommandAstBody::Arguments { arguments } => arguments,
            _ => panic!("Expected arguments"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandAstNode {
    pub content: CommandAstBody,
    pub left: Option<Node>,
    pub right: Option<Node>,
}

#[derive(Debug, Clone)]
pub struct Node(pub Box<CommandAstNode>);

impl Node {
    fn orphan(content: CommandAstBody) -> Node {
        Node(Box::new(CommandAstNode {
            content,
            left: None,
            right: None,
        }))
    }

    fn append_left(&mut self, node: Node) {
        if let Some(left) = self.0.left.as_mut() {
            left.append_left(node);
        } else {
            self.0.left = Some(node);
        }
    }

    fn pipe(left: Node, right: Node) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Pipe,
            left: Some(left),
            right: Some(right),
        }))
    }

    fn separator(left: Option<Node>, right: Option<Node>) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Separator,
            left: left,
            right: right,
        }))
    }
}

impl CommandAstNode {
    fn parse_one(lexer: &mut Lexer<Token>) -> Result<(Option<Node>, Option<Token>)> {
        let mut current = None;
        let mut arguments = Vec::new();
        let mut rest = None;
        while let Some(token) = lexer.next() {
            match token {
                Token::Ident if current.is_none() => {
                    current = Some(Node::orphan(CommandAstBody::Call {
                        name: lexer.slice().to_string(),
                    }));
                }
                Token::Ident
                | Token::LiteralString
                | Token::LiteralInteger
                | Token::LiteralFloat
                    if current.is_some() =>
                {
                    arguments.push(lexer.slice().to_string())
                }
                r => {
                    rest = Some(r);
                    break;
                }
            }
        }
        Ok((
            current.map(|mut e| {
                e.append_left(Node::orphan(CommandAstBody::Arguments { arguments }));
                e
            }),
            rest,
        ))
    }

    pub fn parse(mut lexer: Lexer<Token>) -> Result<Node> {
        pub fn inner_parse(
            mut lexer: Lexer<Token>,
            prev: Node,
            prev_token: Option<Token>,
        ) -> Result<Node> {
            match prev_token.or_else(|| lexer.next()) {
                Some(Token::Pipe) => {
                    let (right, rest) = CommandAstNode::parse_one(&mut lexer)?;
                    inner_parse(
                        lexer,
                        Node::pipe(
                            prev,
                            right.ok_or_else(|| {
                                Error::Parsing(format!("Expected token after pipe"), 0..0)
                            })?,
                        ),
                        rest,
                    )
                }
                Some(Token::Separator) => {
                    let (right, rest) = CommandAstNode::parse_one(&mut lexer)?;
                    inner_parse(lexer, Node::separator(Some(prev), right), rest)
                }
                Some(token) => {
                    return Err(Error::Parsing(
                        format!("Unexpected token {:?} = `{}`", token, lexer.slice()),
                        lexer.span(),
                    ));
                }
                None => Ok(prev),
            }
        }
        let (root, prev) = Self::parse_one(&mut lexer)?;
        dbg!(inner_parse(lexer, root.ok_or(Error::NoData)?, prev))
    }
}
