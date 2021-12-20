use std::collections::BTreeMap;
use std::collections::HashSet;

use super::ast::*;
use super::lexer::*;
use super::ProgramOutput;
use crate::prelude::*;

#[derive(Debug)]
pub struct AstContext {
    scoop_counter: usize,
    declaration_counter: usize,
    memory: BTreeMap<Reference, RuntimeValue>,
    scoops: BTreeMap<ScoopID, NodeScoop>,
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

#[derive(Clone)]
pub enum RuntimeValue {
    Undefined,
    Number(f64),
    String(String),
    Object(BTreeMap<String, RuntimeValue>),
    Array(Vec<RuntimeValue>),
    Procedure(Node),
    NativeProcedure(Arc<Mutex<NativeProcedureGen>>),
}

impl std::fmt::Debug for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            RuntimeValue::Undefined => write!(f, "Undefined"),
            RuntimeValue::Number(payload) => write!(f, "{:?}", payload),
            RuntimeValue::String(payload) => write!(f, "{:?}", payload),
            RuntimeValue::Object(payload) => write!(f, "{:?}", payload),
            RuntimeValue::Array(payload) => write!(f, "{:?}", payload),
            RuntimeValue::Procedure(payload) => write!(f, "{:?}", payload),
            RuntimeValue::NativeProcedure(_payload) => write!(f, "[native]")
        }
    }
}

pub type NativeProcedureGen = Box<
    dyn (Fn(
            Reactor,
            Vec<String>,
            Option<Receiver<ProgramOutput>>,
            Sender<ProgramOutput>,
        ) -> NativeProcedure)
        + Send,
>;
pub type NativeProcedure = Pin<Box<dyn Future<Output = Result<ProgramOutput>> + Send>>;

pub type Reference = usize;
pub type ScoopID = usize;

impl AstContext {
    pub fn new<F: (FnOnce(&mut Self))>(root: &mut Node, init: F) -> Self {
        let mut context = Self {
            declaration_counter: 0,
            scoop_counter: 0,
            memory: BTreeMap::new(),
            scoops: BTreeMap::new(),
        };
        let main_scoop = context.create_scoop(None);
        context
            .aggregate_scoop(root, main_scoop)
            .expect("Failed to aggregate");

        (init)(&mut context);

        context.aggregate_deps(root).expect("Failed to aggregate");
        context
    }

    pub fn scoop_set(
        &mut self,
        scoop: ScoopID,
        label: &str,
        value: RuntimeValue,
    ) -> Result<Reference> {
        let reference = self.new_ref();
        let scoop = self
            .scoops
            .get_mut(&scoop)
            .ok_or_else(|| Error::ScoopNotFound(scoop))?;
        let reference =  if let Some(reference) = scoop.owned_references.get(label) {
            *reference
        } else {
            scoop.owned_references.insert(label.to_string(), reference);
            reference
        };
        self.memory_set(reference, value);
        Ok(reference)

    }

    pub fn scoop_get(&self, scoop: ScoopID, label: &str) -> Option<&'_ RuntimeValue> {
        self.memory_get(*self.scoops.get(&scoop)?.owned_references.get(label)?)
    }

    pub fn scoop_get_mut(&mut self, scoop: ScoopID, label: &str) -> Option<&'_ mut RuntimeValue> {
        self.memory_get_mut(*self.scoops.get(&scoop)?.owned_references.get(label)?)
    }

    pub fn memory_set(&mut self, reference: Reference, value: RuntimeValue) {
        let _ = self.memory.insert(reference, value);
        dbg!(&self.memory);
    }

    pub fn memory_get(&'_ self, reference: Reference) -> Option<&'_ RuntimeValue> {
        self.memory.get(&reference)
    }

    pub fn memory_get_mut(&'_ mut self, reference: Reference) -> Option<&'_ mut RuntimeValue> {
        self.memory.get_mut(&reference)
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
                    self.aggregate_deps(left)?;
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
                let left = node.0.left.as_mut().unwrap();
                let name = left.span().to_string();
                left.0.meta.reference_to = Some(node.0.meta.scoop);
                self.scoops
                    .get_mut(&node.0.meta.scoop)
                    .expect("Parent scoop")
                    .owned_references
                    .insert(name, reference);
                if let Some(right) = node.0.right.as_mut() {
                    self.aggregate_deps(right)?;
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
                return Ok(());
            }

            if let Some(super_parent) = parent.parent {
                parent_scoop = super_parent;
            } else {
                return Err(Error::ReferenceNotFound(span.to_string()));
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
        Self {
            scoop: 1,
            reference_to: None,
        }
    }
}

impl RuntimeValue {
    pub fn binding(generator: NativeProcedureGen) -> Self {
        Self::NativeProcedure(Arc::new(Mutex::new(generator)))
    }
}
