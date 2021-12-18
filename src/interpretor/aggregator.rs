use crate::prelude::*;
use super::ast::*;
use super::lexer::*;

pub struct AstContext {
    declaration_counter: usize,
    memory: HashMap<Reference, ContextCell>,
}

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(f64),
    String(String),
    Object(HashMap<String, RuntimeValue>),
    Array(Vec<RuntimeValue>),
    Closure(Node),
}

pub struct ContextCell {
    owner: ContextID,
    data: RuntimeValue,
}

pub type Reference = usize;
pub type ContextID = usize;

impl AstContext {
    fn new(root: &mut Node) -> Result<Self> {
        let mut context = Self {
            declaration_counter: 0,
            memory: HashMap::new(),
        };
        context.aggregate(root)?;
        Ok(context)
    }

    pub fn aggregate(&mut self, node: &mut Node) -> Result<()> {

        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct NodeContext {
    pub references: HashMap<Reference, RuntimeValue>
}

impl NodeContext {
    pub fn undeterminated() -> Self {
        Self {
            references: HashMap::new(),
        }
    }

}