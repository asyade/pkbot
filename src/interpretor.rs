use crate::prelude::*;
use logos::Logos;

pub mod aggregator;
pub mod ast;
mod lexer;
use ast::*;
use lexer::*;

use self::aggregator::AstContext;

#[derive(Debug, Clone)]
pub enum ProgramOutput {
    Exit {
        message: Option<String>,
        status: ProgramStatus,
    },
    Text {
        message: String,
    },
    Json {
        content: RuntimeValue,
    },
}

impl ProgramOutput {
    pub fn json(ser: RuntimeValue) -> Self {
        ProgramOutput::Json {
            content: ser,
        }
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
    pub context: AstContext,
    pub status: ProgramStatus,
}

impl Program {
    pub fn new<T: AsRef<str>>(text: T) -> Result<Program> {
        let mut root = CommandAstNode::parse(&mut Token::lexer(text.as_ref()), None)?;
        let context = AstContext::new(&mut root, |context| {
            context
                .scoop_set(1, "ls", RuntimeValue::binding(crate::reactor::runtime::ls::wrap()))
                .expect("Failed to register buitlin");
        });
        Ok(Program {
            root,
            status: ProgramStatus::None,
            context,
        })
    }
}
