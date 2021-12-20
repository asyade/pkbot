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
            //CommandAstBody::Literal { .. } => unimplemented!(),
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

    fn assignation(
        reactor: Reactor,
        root: Node,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
        context: SyncContext,
    ) -> JoinHandle<()> {
        tokio::spawn((async || {})())
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
            let lock = context.read().await;
            let value = lock.scoop_get(scoop, name).unwrap();
            match value {
                RuntimeValue::NativeProcedure(gen) => {
                    let fut = (gen.lock().await)(reactor, vec![], stdin, stdout);
                    drop(lock);
                    dbg!(fut.await);
                },
                _ => unimplemented!()
            }
        })())
        //let program_name = root.call_name();
        //let arguments = root
        //    .0
        //    .left
        //    .expect("Expected arguments in left of call node, found None")
        //    .0
        //    .content
        //    .arguments();
        //
        //match program_name.as_str() {
        //    "ls" => tokio::spawn(try_builtin(
        //        ls::main(reactor.clone(), arguments, stdin, stdout.clone()),
        //        stdout,
        //    )),
        //    "cat" => tokio::spawn(try_builtin(
        //        cat::main(reactor.clone(), arguments, stdin, stdout.clone()),
        //        stdout,
        //    )),
        //    "sleep" => tokio::spawn(try_builtin(
        //        sleep::main(reactor.clone(), arguments, stdin, stdout.clone()),
        //        stdout,
        //    )),
        //    "echo" => tokio::spawn(try_builtin(
        //        echo::main(reactor.clone(), arguments, stdin, stdout.clone()),
        //        stdout,
        //    )),
        //    _ => {
        //        let name = program_name.clone();
        //        tokio::spawn((async move || {
        //            let _ = stdout
        //                .send(ProgramOutput::Exit {
        //                    message: Some(format!("Unknown command: {}", &name)),
        //                    status: ProgramStatus::Error,
        //                })
        //                .await
        //                .map_err(|e| {
        //                    log::error!("Failed to send output of buitlin to stdout: ERROR={}", e)
        //                });
        //        })())
        //    }
        //}
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
