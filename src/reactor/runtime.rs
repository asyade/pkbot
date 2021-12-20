mod builtin;
use super::*;
use crate::interpretor::{aggregator::AstContext, ast::*};
pub use builtin::*;

const CHANN_SIZE_PIPLINE: usize = 512;
const CHANN_SIZE_MAIN: usize = 2048;

pub struct ProgramRuntime {
    pub id: ProgramIdentifier,
    pub stdout: Option<Receiver<ProgramOutput>>,
}

macro_rules! inner_spawn {
    ($reactor:expr => $node:expr, $stdin:expr, $stdout:expr) => {
        match $node.0.content {
            CommandAstBody::Call { .. } => ProgramRuntime::call($reactor, $node, $stdin, $stdout),
            // CommandAstBody::Assignation { .. } => unimplemented!(),
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
            //CommandAstBody::Comma => {
            //    ProgramRuntime::separator($node.0.left, $node.0.right, $reactor, $stdin, $stdout)
            //}
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
    pub async fn spawn(mut program: Program, reactor: Reactor) -> ProgramRuntime {
        let (main_sender, main_receiver) = channel(CHANN_SIZE_MAIN);
        let id = reactor.process_counter.fetch_add(1, Ordering::SeqCst);
        Self::inner_spawn(program.root, reactor, None, main_sender).await;
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
    ) -> JoinHandle<()> {
        inner_spawn!(reactor => root, stdin, stdout)
    }

    fn call(
        reactor: Reactor,
        root: Node,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
    ) -> JoinHandle<()> {
        unimplemented!()
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
        unimplemented!()
    }

    fn separator(
        left: Option<Node>,
        right: Option<Node>,
        reactor: Reactor,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
    ) -> JoinHandle<()> {
        // tokio::spawn((async move || {
        // if let Some(left) = left {
        // let _ = tokio::join!(inner_spawn!(reactor.clone() => left, stdin, stdout.clone()));
        // }
        // if let Some(right) = right {
        // let _ = tokio::join!(inner_spawn!(reactor => right, None, stdout.clone()));
        // }
        // })())
        unimplemented!()
    }
}
