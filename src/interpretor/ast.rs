use super::aggregator::NodeContext;
use super::Token;
use crate::prelude::*;
use logos::Lexer;
use ptree::*;

#[derive(IsVariant, Debug, Clone, Display)]
pub enum CommandAstBody {
    #[display(fmt = "Call")]
    Call,
    #[display(fmt = "Declare")]
    Declare,
    #[display(fmt = "FnArgs")]
    FnArguments,
    #[display(fmt = "Block")]
    Block,
    #[display(fmt = "CallArgs")]
    CallArguments,
    #[display(fmt = "Literal")]
    Literal { token: Token, value: String },
    #[display(fmt = "Ident")]
    Ident { span: String },
    #[display(fmt = "Closure")]
    Closure,
    #[display(fmt = "Assign")]
    Assignation,
    #[display(fmt = "Pipe")]
    Pipe,
    #[display(fmt = "Comma")]
    Comma,
}

#[derive(Debug, Clone)]
pub struct CommandAstNode {
    pub meta: NodeContext,
    pub content: CommandAstBody,
    pub left: Option<Node>,
    pub right: Option<Node>,
}

#[derive(Debug, Clone)]
pub struct Node(pub Box<CommandAstNode>);

macro_rules! expected_token {
    ($what:expr, $after:expr, $found:expr, $lexer:expr) => {
        match $found {
            Some(_tok) => Err(Error::Parsing(
                format!(
                    "Expected {} after {}, found `{}`",
                    $what,
                    $after,
                    $lexer.slice()
                ),
                $lexer.span(),
            )),
            None => Err(Error::Parsing(
                format!("Expected {} after {}", $what, $after),
                $lexer.span(),
            )),
        }
    };
    ($what:expr, $after:expr, $found:expr) => {
        match $found {
            Some(_tok) => Err(Error::Parsing(
                format!("Expected {} after {}, found `{}`", $what, $after, _tok),
                0..0,
            )),
            None => Err(Error::Parsing(
                format!("Expected {} after {}", $what, $after),
                0..0,
            )),
        }
    };
}

impl CommandAstNode {
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
                Some(Token::Assign) => match CommandAstNode::parse_one(lexer)? {
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
                },
                Some(Token::Comma) => {
                    let right = CommandAstNode::parse(lexer, scope)?;
                    inner_parse(lexer, Node::comma(Some(prev), Some(right)), None, scope)
                }
                Some(Token::BraceClose) if scope.is_some() => Ok(prev),
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
        inner_parse(lexer, root.ok_or(Error::NoData)?, prev, scope)
    }

    fn parse_one(lexer: &mut Lexer<Token>) -> Result<(Option<Node>, Option<Token>)> {
        match lexer.next() {
            None => Ok((None, None)),
            Some(token) if token.is_literal() => {
                Ok((Some(Node::literal(token, lexer.slice().to_string())), None))
            }
            Some(Token::GroupOpen) => Self::parse_closure(lexer).map(|(e, r)| (Some(e), r)),
            Some(Token::BraceOpen) => {
                let node = Self::parse(lexer, Some(()))?;
                Ok((Some(node), None))
            }
            Some(Token::Keyword) => match lexer.slice() {
                "let" => match lexer.next() {
                    Some(tok) if tok.is_ident() => {
                        let (ident, rest) = Self::parse_ident(lexer)?;
                        match rest.or_else(|| lexer.next()) {
                            Some(token) if token.is_assign() || token.is_comma() => {
                                Ok((Some(Node::declare(ident)), Some(token)))
                            }
                            e => expected_token!("identifier", "let keyword", e, lexer),
                        }
                    }
                    e => expected_token!("identifier", "let keyword", e, lexer),
                },
                _ => Err(Error::Parsing(
                    format!("Unexpected identifier `{}`", lexer.slice()),
                    lexer.span(),
                )),
            },
            Some(Token::Ident) => {
                let (target_node, rest) = Self::parse_ident(lexer)?;
                let mut node = Node::orphan(CommandAstBody::Call);
                node.0.right = Some(target_node);
                match rest.or_else(|| lexer.next()) {
                    Some(Token::GroupOpen) => {
                        let (arguments, rest) =
                            Self::parse_arguments(lexer, None, CommandAstBody::CallArguments)?;
                        match rest {
                            Some(Token::GroupClose) => {
                                node.0.left = Some(arguments);
                                Ok((Some(node), None))
                            }
                            e => expected_token!("`)`", "call", e, lexer),
                        }
                    }
                    rest => Ok((Some(node), rest)),
                }
            }
            r => Ok((None, r)),
        }
    }

    fn parse_closure(lexer: &mut Lexer<Token>) -> Result<(Node, Option<Token>)> {
        let (arguments, rest) = Self::parse_arguments(lexer, None, CommandAstBody::FnArguments)?;
        match rest {
            Some(Token::GroupClose) => match (lexer.next(), lexer.next()) {
                (Some(Token::Fn), Some(Token::BraceOpen)) => {
                    let node = Self::parse(lexer, Some(())).map(|e| Some(e)).or_else(|e| {
                        if let Error::NoData = e {
                            Ok(None)
                        } else {
                            Err(e)
                        }
                    })?;
                    let scope = Node::block(node);
                    Ok((Node::closure(arguments, scope), None))
                }
                _ => Err(Error::Parsing(format!("Expected `=>``"), lexer.span())),
            },
            e => expected_token!("`)`", "closure arguments list", e, lexer),
        }
    }

    fn parse_arguments(
        lexer: &mut Lexer<Token>,
        mut rest: Option<Token>,
        body: CommandAstBody,
    ) -> Result<(Node, Option<Token>)> {
        fn parse_one_argument(
            lexer: &mut Lexer<Token>,
            mut rest: Option<Token>,
        ) -> Result<(Option<Node>, Option<Token>)> {
            match rest.take().or_else(|| lexer.next()) {
                Some(Token::Ident) => {
                    Ok(CommandAstNode::parse_ident(lexer).map(|(a, b)| (Some(a), b))?)
                }
                Some(Token::GroupOpen) => {
                    CommandAstNode::parse_closure(lexer).map(|(e, r)| (Some(e), r))
                }
                Some(tok) if tok.is_literal() => {
                    Ok((Some(Node::literal(tok, lexer.slice().to_string())), None))
                }
                tok => Ok((None, tok)),
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
                            continue;
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
                }
                Some(Token::Ident) if expect_ident => {
                    main.append_left(Node::ident(lexer.slice().to_string()));
                }
                Some(Token::Ident) => {
                    return Err(Error::Parsing(
                        format!("Unexpected identifier"),
                        lexer.span(),
                    ))
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
            ..Default::default()
        }))
    }

    fn declare(ident: Node) -> Node {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Declare,
            left: Some(ident),
            ..Default::default()
        }))
    }

    fn literal(token: Token, value: String) -> Node {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Literal { token, value },
            ..Default::default()
        }))
    }

    fn ident(span: String) -> Node {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Ident { span },
            ..Default::default()
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
            ..Default::default()
        }))
    }

    fn pipe(left: Node, right: Node) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Pipe,
            left: Some(left),
            right: Some(right),
            ..Default::default()
        }))
    }

    fn block(body: Option<Node>) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Block,
            left: body,
            ..Default::default()
        }))
    }

    fn comma(left: Option<Node>, right: Option<Node>) -> Self {
        Node(Box::new(CommandAstNode {
            content: CommandAstBody::Comma,
            left: left,
            right: right,
            ..Default::default()
        }))
    }

    fn assignation(left: Node, right: Node) -> Result<Self> {
        match (&left.0.content, left.0.right.as_ref().map(|e| &e.0.content)) {
            (CommandAstBody::Declare, _) => Ok(Node(Box::new(CommandAstNode {
                content: CommandAstBody::Assignation,
                left: Some(left),
                right: Some(right),
                ..Default::default()
            }))),
            (CommandAstBody::Call, Some(CommandAstBody::Ident { .. })) if left.0.left.is_none() => {
                Ok(Node(Box::new(CommandAstNode {
                    content: CommandAstBody::Assignation,
                    left: left.0.right,
                    right: Some(right),
                    ..Default::default()
                })))
            }
            (body, _) => {
                expected_token!("expression", "assignation", Some(body))
            }
        }
    }

    pub fn span(&'_ self) -> &'_ str {
        match &self.0.content {
            CommandAstBody::Ident { span } => &span,
            _ => panic!("Not an identifier"),
        }
    }
}

impl Default for CommandAstNode {
    fn default() -> Self {
        CommandAstNode {
            content: CommandAstBody::Block,
            meta: NodeContext::undeterminated(),
            left: None,
            right: None,
        }
    }
}

impl<'a> TreeItem for &'a Node {
    type Child = Self;

    fn write_self<W: std::io::Write>(&self, f: &mut W, style: &Style) -> std::io::Result<()> {
        match &self.0.meta {
            NodeContext {
                scoop,
                reference_to: Some(reference_to),    
            } => write!(
                f,
                "{}",
                style.paint(format!("{} ({} -> {})", &self.0.content, scoop, reference_to))
            ),
            NodeContext {
                scoop,
                ..
            } => write!(
                f,
                "{}",
                style.paint(format!("{} ({})", &self.0.content, scoop))
            )
    
        }
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
