use std::collections::BTreeMap;
use std::collections::HashSet;

use super::ast::*;
use super::lexer::*;
use crate::prelude::*;

#[derive(Debug)]
pub struct AstContext {
    scoop_counter: usize,
    declaration_counter: usize,
    memory: BTreeMap<Reference, ContextCell>,
    scoops: BTreeMap<ScoopID, NodeScoop>,
}

#[derive(Debug)]
pub struct ContextCell {
    data: RuntimeValue,
}

#[derive(Debug, Clone)]
pub struct NodeScoop {
    parent: Option<ScoopID>,
    children: HashSet<ScoopID>,
    owned_references: BTreeMap<String, Reference>,
}

#[derive(Debug, Clone)]
pub struct NodeContext {
    pub scoop: ScoopID,
    pub reference_to: Option<ScoopID>,
}

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(f64),
    String(String),
    Object(HashMap<String, RuntimeValue>),
    Array(Vec<RuntimeValue>),
    Closure(Node),
}

pub type Reference = usize;
pub type ScoopID = usize;


impl AstContext {
    pub fn new(root: &mut Node) -> Self {
        let mut context = Self {
            declaration_counter: 0,
            scoop_counter: 0,
            memory: BTreeMap::new(),
            scoops: BTreeMap::new(),
        };
        let main_scoop = context.create_scoop(None);
        context.aggregate_scoop(root, main_scoop).expect("Failed to aggregate");
        context.aggregate_deps(root).expect("Failed to aggregate");
        context
    }

    fn create_scoop(&mut self, parent: Option<ScoopID>) -> ScoopID {
        self.scoop_counter += 1;
        let id = self.scoop_counter;
        self.scoops.insert(id, NodeScoop::new(parent));
        if let Some(parent) = parent {
            self.scoops
                .get_mut(&parent)
                .expect("Can't find parent scoop")
                .children
                .insert(id);
        }
        id
    }

    fn new_ref(&mut self) -> Reference {
        self.declaration_counter += 1;
        self.declaration_counter
    }

    fn aggregate_scoop(&mut self, node: &mut Node, parent_scoop: ScoopID) -> Result<()> {
        match &node.0.content {
            CommandAstBody::CallArguments
            | CommandAstBody::FnArguments
            | CommandAstBody::Literal { .. }
            | CommandAstBody::Closure
            | CommandAstBody::Ident { .. }
            | CommandAstBody::Comma
            | CommandAstBody::Call
            | CommandAstBody::Declare
            | CommandAstBody::Assignation
            | CommandAstBody::Pipe => {
                node.0.meta.scoop = parent_scoop;
                if let Some(left) = node.0.left.as_mut() {
                    self.aggregate_scoop(left, parent_scoop)?;
                }
                if let Some(right) = node.0.right.as_mut() {
                    self.aggregate_scoop(right, parent_scoop)?;
                }
                Ok(())
            }
            CommandAstBody::Block => {
                let new_scoope = self.create_scoop(Some(parent_scoop));
                node.0.meta.scoop = new_scoope;
                if let Some(left) = node.0.left.as_mut() {
                    self.aggregate_scoop(left, new_scoope)?;
                }
                if let Some(right) = node.0.right.as_mut() {
                    self.aggregate_scoop(right, new_scoope)?;
                }
                Ok(())
            }
        }
    }

    fn aggregate_deps(&mut self, node: &mut Node) -> Result<()> {
        match &node.0.content {
            CommandAstBody::CallArguments
            | CommandAstBody::FnArguments
            | CommandAstBody::Closure
            | CommandAstBody::Block
            | CommandAstBody::Literal { .. }
            | CommandAstBody::Ident { .. }
            | CommandAstBody::Comma
            | CommandAstBody::Pipe => {
                if let Some(left) = node.0.left.as_mut() {
                    self.aggregate_deps(left,)?;
                }
                if let Some(right) = node.0.right.as_mut() {
                    self.aggregate_deps(right)?;
                }
                Ok(())
            }
            CommandAstBody::Call => {
                if let Some(left) = node.0.left.as_mut() {
                    self.aggregate_deps(left)?;
                }
                if node.0.right.as_ref().unwrap().0.content.is_ident() {
                    self.aggregate_reference(node.0.right.as_mut().unwrap())?;
                } else {
                    self.aggregate_deps(node.0.right.as_mut().unwrap())?;
                }
                Ok(())
            }
            CommandAstBody::Assignation => {
                if node.0.left.as_ref().unwrap().0.content.is_ident() {
                    self.aggregate_reference(node.0.left.as_mut().unwrap())?;
                } else {
                    self.aggregate_deps(node.0.left.as_mut().unwrap())?;
                }
                if let Some(right) = node.0.right.as_mut() {
                    self.aggregate_deps(right)?;
                }
                Ok(())
            }
            CommandAstBody::Declare => {
                let reference = self.new_ref();
                let name = node.0.left.as_ref().unwrap().span().to_string();
                self.scoops
                    .get_mut(&node.0.meta.scoop)
                    .expect("Parent scoop")
                    .owned_references
                    .insert(name, reference);
                if let Some(left) = node.0.left.as_mut() {
                    self.aggregate_deps(left)?;
                }
                Ok(())
            }
        }
    }


    fn aggregate_reference(&mut self, node: &mut Node) -> Result<()> {
        let mut parent_scoop = node.0.meta.scoop;
        let span = node.span();
        loop {
            let parent = &self.scoops[&parent_scoop];
            if parent.owned_references.contains_key(span) {
                node.0.meta.reference_to = Some(parent_scoop);
                return Ok(())
            }

            if let Some(super_parent) = parent.parent {
                parent_scoop = super_parent;
            } else {
                return Err(Error::ReferenceNotFound(span.to_string()))
                // return Err(unimplemented!())
            }
        }
    }
}

impl NodeScoop {
    fn new(parent: Option<usize>) -> Self {
        Self {
            parent,
            children: HashSet::new(),
            owned_references: BTreeMap::new(),
        }
    }
}

impl NodeContext {
    pub fn undeterminated() -> Self {
        Self { scoop: 1, reference_to: None }
    }
}
