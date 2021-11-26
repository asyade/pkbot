mod builtin;
use super::*;
use crate::interpretor::ast::*;
pub use builtin::*;

const CHANN_SIZE_PIPLINE: usize = 512;
const CHANN_SIZE_MAIN: usize = 2048;

pub struct ProgramRuntime {
    pub id: ProgramIdentifier,
    pub stdout: Option<Receiver<ProgramOutput>>,
}

impl ProgramRuntime {
    pub async fn spawn(root: Node, reactor: Reactor) -> ProgramRuntime {
        let (main_sender, main_receiver) = channel(CHANN_SIZE_MAIN);
        let id = reactor.process_counter.fetch_add(1, Ordering::SeqCst);
        Self::inner_spawn(root, reactor, None, main_sender).await;
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
    ) {
        match &root.0.content {
            CommandAstBody::Call { .. } => {
                tokio::spawn(Self::call(reactor, root, stdin, stdout));
            }
            CommandAstBody::Pipe if root.0.left.is_some() && root.0.right.is_some() => {
                Self::pipe(
                    root.0.left.unwrap(),
                    root.0.right.unwrap(),
                    reactor,
                    stdin,
                    stdout,
                )
                .await;
            }
            CommandAstBody::Separator => {
                Self::separator(root.0.left, root.0.right, reactor, stdin, stdout).await;
            }
            CommandAstBody::Pipe => {
                panic!("Invalide pipe");
            }
            CommandAstBody::Arguments { .. } => {
                panic!("Expected call or pipe, found arguments");
            }
        }
    }

    async fn call(
        reactor: Reactor,
        root: Node,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
    ) {
        let program_name = root.0.content.call_name();
        let arguments = root
            .0
            .left
            .expect("Expected arguments in left of call node, found None")
            .0
            .content
            .arguments();


        let _handle = match program_name.as_str() {
            "ls" => tokio::spawn(ls::main(reactor.clone(), arguments, stdin, stdout)),
            "cat" => tokio::spawn(cat::main(reactor.clone(), arguments, stdin, stdout)),
            "sleep" => tokio::spawn(sleep::main(reactor.clone(), arguments, stdin, stdout)),
            _ => {
                let name = program_name.clone();
                tokio::spawn((async move || {
                    let _ = stdout.send(ProgramOutput::Exit {
                        message: Some(format!("Unknown command: {}", &name)),
                        status: ProgramStatus::Error,
                    }).await.map_err(|e| log::error!("Failed to send output of buitlin to stdout: ERROR={}", e));
                })())
            }
        };
    }

    async fn pipe(
        left: Node,
        right: Node,
        reactor: Reactor,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
    ) {
        let (pipline_sender, pipline_receiver) = channel(CHANN_SIZE_PIPLINE);
        let _left = Self::call(reactor.clone(), left, stdin, pipline_sender).await;
        let _right = Self::call(reactor.clone(), right, Some(pipline_receiver), stdout).await;
    }

    async fn separator(
        left: Option<Node>,
        right: Option<Node>,
        _reactor: Reactor,
        stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
    ) {
        // let _left = Self::call(left, stdin, pipline_sender).await;
        // let _right = Self::call(right, Some(pipline_receiver), stdout).await;
    }
}
