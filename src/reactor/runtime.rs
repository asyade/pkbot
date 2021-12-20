mod builtin;
use super::*;
use crate::interpretor::{
    aggregator::{AstContext, NodeContext},
    ast::*,
};
use crate::prelude::*;
pub use builtin::*;

pub type SyncContext = Arc<RwLock<AstContext>>;

const CHANN_SIZE_PIPLINE: usize = 512;
const CHANN_SIZE_MAIN: usize = 2048;

pub struct ProgramRuntime {
    pub id: ProgramIdentifier,
    pub stdout: Option<Receiver<ProgramOutput>>,
}

macro_rules! inner_spawn {
    ($reactor:expr => $node:expr, $stdin:expr, $stdout:expr, $context:expr) => {
        match $node.0.content {
            CommandAstBody::Call { .. } => {
                ProgramRuntime::call($reactor, $node, $stdin, $stdout, $context)
            }
            CommandAstBody::Assignation { .. } => {
                ProgramRuntime::assignation($reactor, $node, $stdin, $stdout, $context)
            }
            CommandAstBody::Literal { .. } => unimplemented!(),
            //CommandAstBody::Pipe if $node.0.left.is_some() && $node.0.right.is_some() => {
            //    ProgramRuntime::pipe(
            //        $node.0.left.unwrap(),
            //        $node.0.right.unwrap(),
            //        $reactor,
            //        $stdin,
            //        $stdout,
            //    )
            //}
            CommandAstBody::Comma => ProgramRuntime::separator(
                $node.0.left,
                $node.0.right,
                $reactor,
                $stdin,
                $stdout,
                $context,
            ),
            //CommandAstBody::Pipe => {
            //    panic!("Invalide pipe");
            //}
            //CommandAstBody::Arguments { .. } => {
            //    panic!("Expected call or pipe, found arguments");
            //}
            _ => unimplemented!(),
        }
    };
}

impl ProgramRuntime {
    pub async fn spawn(program: Program, reactor: Reactor) -> ProgramRuntime {
        let (main_sender, main_receiver) = channel(CHANN_SIZE_MAIN);
        let context = Arc::new(RwLock::new(program.context));
        let id = reactor.process_counter.fetch_add(1, Ordering::SeqCst);
        Self::inner_spawn(program.root, reactor, None, main_sender, context).await;
        ProgramRuntime {
            id,
            stdout: Some(main_receiver),
        }
    }

    async fn inner_spawn(
        root: Node,
        reactor: Reactor,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
        context: SyncContext,
    ) -> JoinHandle<()> {
        inner_spawn!(reactor => root, stdin, stdout, context)
    }

    //TODO: optime
    fn assignation(
        reactor: Reactor,
        root: Node,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
        context: SyncContext,
    ) -> JoinHandle<()> {
        tokio::spawn((async move || {


            let (pipline_sender, mut pipline_receiver) = channel(CHANN_SIZE_PIPLINE);
            let left = root.0.left.expect("Left operand");
            let call = root.0.right.expect("Right operand"); 

            if call.0.content.is_closure() {
                let (target_name, target_scoop) = left.reference();
                if let Err(e) = context.write().await.scoop_set(target_scoop, target_name, RuntimeValue::Procedure(call)) {
                    dbg!(e);
                }
            } else {
                let handle = inner_spawn!(reactor => call, stdin, pipline_sender, context.clone());
                let mut values = Vec::with_capacity(1);
                while let Some(res) = pipline_receiver.recv().await {
                    match res {
                        ProgramOutput::Json { content } => values.push(content),
                        _ => {}, 
                    }
                }
                let _ = handle.await;
                drop(stdout);
                let value = if values.len() > 0 {
                    RuntimeValue::Array(values)
                } else if values.len() == 1 {
                    values.remove(0)
                } else {
                    RuntimeValue::Undefined
                };
                let (target_name, target_scoop) = left.reference();
                if let Err(e) = context.write().await.scoop_set(target_scoop, target_name, value) {
                    dbg!(e);
                }
            }
        })())
    }

    fn call(
        reactor: Reactor,
        root: Node,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
        context: SyncContext,
    ) -> JoinHandle<()> {
        tokio::spawn((async move || { 
            let (name, scoop) = root.0.right.as_ref().unwrap().reference();
            let ctx = context.clone();
            let lock = context.read().await;
            let value = lock.scoop_get(scoop, name).unwrap();
            match value {
                RuntimeValue::NativeProcedure(gen) => {
                    let fut = (gen.lock().await)(reactor, vec![], stdin, stdout.clone());
                    drop(lock);
                    if let Ok(res) = fut.await {
                        let _ = stdout.send(res).await;
                    }
                },
                RuntimeValue::Procedure(node) => {
                    if let Some(right) = node.0.right.as_ref().unwrap().0.left.clone() {
                        drop(lock);
                        let fut = inner_spawn!(reactor => right, stdin, stdout, ctx);
                        let _ = fut.await;
                    }
                },
                _ => unimplemented!()
            }
        })())
    }

    fn pipe(
        left: Node,
        right: Node,
        reactor: Reactor,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
    ) -> JoinHandle<()> {
        // tokio::spawn((async move || {
        // let (pipline_sender, pipline_receiver) = channel(CHANN_SIZE_PIPLINE);
        // let _left = inner_spawn!(reactor.clone() => left, stdin, pipline_sender);
        // let _ = inner_spawn!(reactor.clone() => right, Some(pipline_receiver), stdout.clone());
        // })())
        tokio::spawn((async || {})())
    }

    fn separator(
        left: Option<Node>,
        right: Option<Node>,
        reactor: Reactor,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
        context: SyncContext,
    ) -> JoinHandle<()> {
        tokio::spawn((async move || {
            if let Some(left) = left {
                let _ = tokio::join!(
                    inner_spawn!(reactor.clone() => left, stdin, stdout.clone(), context.clone())
                );
            }
            if let Some(right) = right {
                let _ = tokio::join!(inner_spawn!(reactor => right, None, stdout.clone(), context));
            }
        })())
    }
}
