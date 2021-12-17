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
    Text {
        message: String,
    },
    Json {
        content: Value,
    },
}

impl ProgramOutput {
    pub fn json<T: Serialize>(ser: T) -> Result<Self> {
        Ok(ProgramOutput::Json {
            content: serde_json::to_value(ser)?,
        })
    }
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
        let root = CommandAstNode::parse(&mut Token::lexer(text.as_ref()), None)?;
        Ok(Program {
            root,
            status: ProgramStatus::None,
        })
    }
}
