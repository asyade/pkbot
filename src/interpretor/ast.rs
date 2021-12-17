use super::Token;
use crate::prelude::*;
use logos::Lexer;
use ptree::*;

#[derive(Debug, Clone)]
pub enum CommandAstBody {
    Call,
    FnArguments,
    CallArguments,
    Literal { token: Token, value: String },
    Ident { span: String },
    Block,
    Closure,
    Assignation,
    Pipe,
    Comma,
}

#[derive(Debug, Clone)]
pub struct CommandAstNode {
    pub content: CommandAstBody,
    pub left: Option<Node>,
    pub right: Option<Node>,
}

#[derive(Debug, Clone)]
pub struct Node(pub Box<CommandAstNode>);

impl CommandAstNode {
    fn parse_one(lexer: &mut Lexer<Token>) -> Result<(Option<Node>, Option<Token>)> {
        match lexer.next() {
            None => Ok((None, None)),
            Some(token) if token.is_literal() => {
                dbg!(&token);
                Ok((Some(Node::literal(token, lexer.slice().to_string())), None))
            },
            Some(Token::GroupOpen) => {
                Self::parse_closure(lexer).map(|(e, r)| (Some(e), r))
            },
            Some(Token::BraceOpen) => {
                let node = Self::parse(lexer, Some(()))?;
                Ok((Some(node), None))
            },
            Some(Token::Ident) => {
                let (target_node, rest) = Self::parse_ident(lexer)?;
                let mut node = Node::orphan(CommandAstBody::Call);
                node.0.right = Some(target_node);
                match rest.or_else(|| lexer.next()) {
                    Some(Token::GroupOpen) => {
                        let (arguments, rest) = Self::parse_arguments(lexer, None, CommandAstBody::CallArguments)?;
                        match rest {
                            Some(Token::GroupClose) => {
                                node.0.left = Some(arguments);
                                Ok((Some(node), None))
                            },
                            Some(_) => Err(Error::Parsing(format!("Expected `)` found `{}`", lexer.slice()), lexer.span())),
                            None => Err(Error::Parsing(format!("Expected `)``"), lexer.span())),
                        }
                    },
                    rest => {
                        Ok((Some(node), rest))
                    },
                }
            },
            r => {
                Ok((None, r))
            }
        }
    }

    pub fn parse(lexer: &mut Lexer<Token>, scope: Option<()>) -> Result<Node> {
        pub fn inner_parse(
            lexer: &mut Lexer<Token>,
            prev: Node,
            prev_token: Option<Token>,
            scope: Option<()>,
        ) -> Result<Node> {
            match prev_token.or_else(|| lexer.next()) {
                Some(Token::Pipe) => {
                    let (right, rest) = CommandAstNode::parse_one(lexer)?;
                    inner_parse(
                        lexer,
                        Node::pipe(
                            prev,
                            right.ok_or_else(|| {
                                Error::Parsing(format!("Expected token after pipe"), 0..0)
                            })?,
                        ),
                        rest,
                        scope,
                    )
                }
                Some(Token::Assign) => {
                    match CommandAstNode::parse_one(lexer)? {
                        (Some(right), rest) => {
                            inner_parse(lexer, Node::assignation(prev, right)?, rest, scope)
                        }
                        (None, Some(token)) if token.is_literal() => {
                            let right = Node::literal(token, lexer.slice().to_string());
                            inner_parse(lexer, Node::assignation(prev, right)?, None, scope)
                        }
                        (None, token) => {
                            return Err(Error::Parsing(
                                format!("Expected expression after assignation, found `{:?}`", token),
                                lexer.span(),
                            ));
                        }
                    }
                }
                Some(Token::Comma) => {
                    let (right, rest) = CommandAstNode::parse_one(lexer)?;
                    inner_parse(lexer, Node::comma(Some(prev), right), rest, scope)
                }
                Some(Token::BraceClose) if scope.is_some() => {
                    Ok(prev)
                },
                Some(token) => {
                    return Err(Error::Parsing(
                        format!("Unexpected token {:?} = `{}`", token, lexer.slice()),
                        lexer.span(),
                    ));
                }
                None => Ok(prev),
            }
        }
        let (root, prev) = Self::parse_one(lexer)?;
        // if let Some(_scope) = scope {
            // inner_parse(lexer, Node::block(root), prev, scope)
        // } else {
            inner_parse(lexer, root.ok_or(Error::NoData)?, prev, scope)
        // }
    }

    fn parse_closure(lexer: &mut Lexer<Token>) -> Result<(Node, Option<Token>)> {
        let (arguments, rest) = Self::parse_arguments(lexer, None, CommandAstBody::FnArguments)?;
        match rest {
            Some(Token::GroupClose) => {
                match (lexer.next(), lexer.next()) {
                    (Some(Token::Fn), Some(Token::BraceOpen)) => {
                        let node = Self::parse(lexer, Some(())).map(|e| Some(e)).or_else(|e| if let Error::NoData = e {Ok(None)} else { Err(e) })?;
                        let scope = Node::block(node);
                        Ok((Node::closure(arguments, scope), None))
                    },
                    _ => Err(Error::Parsing(format!("Expected `=>``"), lexer.span())),
                }
            },
            Some(_) => Err(Error::Parsing(format!("Expected `)` found `{}`", lexer.slice()), lexer.span())),
            None => Err(Error::Parsing(format!("Expected `)``"), lexer.span())),
        }
    }

    fn parse_arguments(lexer: &mut Lexer<Token>, mut rest: Option<Token>, body: CommandAstBody) -> Result<(Node, Option<Token>)> {
        fn parse_one_argument(lexer: &mut Lexer<Token>, mut rest: Option<Token>) -> Result<(Option<Node>, Option<Token>)> {
            match rest.take().or_else(|| lexer.next()) {
                Some(Token::Ident) => {
                    Ok(CommandAstNode::parse_ident(lexer).map(|(a, b)| (Some(a), b))?)
                },
                Some(Token::GroupOpen) => {
                    CommandAstNode::parse_closure(lexer).map(|(e, r)| (Some(e), r))
                }
                Some(tok) if tok.is_literal() => {
                    Ok((Some(Node::literal(tok, lexer.slice().to_string())), None))
                },
                tok => {
                    Ok((None, tok))
                }
            }
        }
        let mut root = Node::orphan(body);
        loop {
            let takken = rest.take();
            match parse_one_argument(lexer, takken)? {
                (Some(argument), r) => {
                    root.append_right(argument);
                    rest = r.or_else(|| lexer.next());
                    if let Some(r) = rest.as_ref() {
                        if let Token::Separator = r {
                            rest.take();
                            continue
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                (None, r) => {
                    rest = r;
                    break;
                }
            }
        }
        Ok((root, rest))

    }

    fn parse_ident(lexer: &mut Lexer<Token>) -> Result<(Node, Option<Token>)> {
        let mut main = Node::ident(lexer.slice().to_string());
        let mut rest = None;
        let mut expect_ident = false;
        loop {
            match lexer.next() {
                Some(Token::Deref) => {
                    expect_ident = true;
                },
                Some(Token::Ident) if expect_ident => {
                    main.append_left(Node::ident(lexer.slice().to_string()));
                },
                Some(Token::Ident) => {
                    return Err(Error::Parsing(format!("Unexpected identifier"), lexer.span()))
                }
                None => break,
                Some(token) => {
                    rest = Some(token);
                    break;
                }
            }
        }
        Ok((main, rest))
    }

}


impl Node {
    fn orphan(content: CommandAstBody) -> Node {
        Node(Box::new(CommandAstNode {
            content,
            left: None,
            right: None,
        }))
    }

    fn literal(token: Token, value: String) -> Node {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Literal{ token, value },
            left: None,
            right: None,
        }))
    }

    fn ident(span: String) -> Node {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Ident { span },
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

    fn append_right(&mut self, node: Node) {
        if let Some(right) = self.0.right.as_mut() {
            right.append_right(node);
        } else {
            self.0.right = Some(node);
        }
    }

    fn closure(arguments: Node, body: Node) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Closure,
            left: Some(arguments),
            right: Some(body),
        }))
    }

    fn pipe(left: Node, right: Node) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Pipe,
            left: Some(left),
            right: Some(right),
        }))
    }

    fn block(body: Option<Node>) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Block,
            left: body,
            right: None,
        }))
    }


    fn comma(left: Option<Node>, right: Option<Node>) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Comma,
            left: left,
            right: right,
        }))
    }

    fn assignation(left: Node, right: Node) -> Result<Self> {
        match (left.0.content, left.0.right.as_ref().map(|e| &e.0.content)) {
            (CommandAstBody::Call, Some(CommandAstBody::Ident{ .. })) if left.0.left.is_none() => {
                Ok(Node(Box::new(CommandAstNode {
                    content: CommandAstBody::Assignation,
                    left: left.0.right,
                    right: Some(right),
                })))
            }
            _e => {
                Err(Error::Parsing("Expected reference on left side of assignation".to_string(), 0..0))
            }
        }
    }

}

impl CommandAstBody {
    pub fn is_call(&self) -> bool {
        if let CommandAstBody::Call = self {
            true
        } else {
            false
        }
    }
}

impl std::fmt::Display for CommandAstBody {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", match self {
            CommandAstBody::Call => "Call",
            CommandAstBody::FnArguments => "FnArgs",
            CommandAstBody::Block => "Block",
            CommandAstBody::CallArguments => "CallArgs",
            CommandAstBody::Literal { .. } => "Literal",
            CommandAstBody::Ident { .. } => "Ident",
            CommandAstBody::Closure => "Closure",
            CommandAstBody::Assignation => "Assign",
            CommandAstBody::Pipe => "Pipe",
            CommandAstBody::Comma => "Comma",        
        })
    }
}

impl <'a> TreeItem for &'a Node {
    type Child = Self;

    fn write_self<W: std::io::Write>(&self, f: &mut W, style: &Style) -> std::io::Result<()> {
        write!(f, "{}", style.paint(format!("{}", &self.0.content)))
    }

    fn children(&self) -> Cow<[Self::Child]> {
        Cow::from(match (self.0.left.as_ref(), self.0.right.as_ref()) {
            (Some(a), Some(b)) => vec![a, b],
            (None, Some(b)) => vec![b],
            (Some(a), None) => vec![a],
            (None, None) => vec![],
        })
    }
}
