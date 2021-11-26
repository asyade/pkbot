use crate::prelude::*;
use logos::Logos;

pub mod ast;
mod lexer;
use ast::*;
use lexer::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgramOutput {
    Exit {
        message: Option<String>,
        status: ProgramStatus,
    },
    Json {
        content: Value,
    },
}

pub type ProgramIdentifier = u64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ProgramStatus {
    None,
    Running,
    Success,
    Error,
}

#[derive(Debug)]
pub struct Program {
    pub root: Node,
    pub status: ProgramStatus,
}

impl Program {
    pub fn new<T: AsRef<str>>(text: T) -> Result<Program> {
        Ok(Program {
            root: CommandAstNode::parse(Token::lexer(text.as_ref()))?,
            status: ProgramStatus::None,
        })
    }
}
