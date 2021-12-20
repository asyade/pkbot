#![feature(async_closure, type_alias_impl_trait)]
use clap::{App, Arg};

pub(crate) mod error;
pub(crate) mod prelude;

pub(crate) mod exchange;
pub(crate) mod interpretor;
pub(crate) mod reactor;
pub(crate) mod store;

// mod api;

use exchange::*;
use interpretor::{Program, ProgramOutput};
use prelude::*;
use reactor::*;

#[rocket::main]
async fn main() {
    pretty_env_logger::init();

    let matches = App::new("pkbot")
        .author("Asya C.")
        .version("0.1")
        .subcommand(App::new("daemon").about("Launch a reactor deamon"))
        .subcommand(
            App::new("ast")
                .about("Print ast of a command")
                .arg(Arg::new("command").required(true)),
        )
        .subcommand(
            App::new("exec")
                .about("Execute a command")
                .arg(Arg::new("command").required(true)),
        )
        .get_matches();

    let store_path = std::env::var("STORE_PATH").expect("STORE_PATH");
    let store = Store::new(PathBuf::from(store_path)).unwrap();
    let reactor = Reactor::new(store.handle()).await;
    let kraken = KrakenExchange::from_env()
        .expect("Kraken credentials")
        .boxed();
    reactor.register_exchange(kraken).await;
    match matches.subcommand_name() {
        Some("daemon") => {
            //api::spawn(reactor)
            //    .await
            //    .expect("Failed to launch api server");
        }
        Some("ast") => {
            let matches = matches.subcommand_matches("ast").unwrap();
            let command = matches.value_of("command").unwrap();
            let program = Program::new(command).expect("Failed to parse command");
            dbg!(&program.context);
            ptree::print_tree(&&program.root).expect("Unable to print directory tree");
        }
        Some("exec") => {
            let matches = matches.subcommand_matches("exec").unwrap();
            let command = matches.value_of("command").unwrap();
            let program = Program::new(command).expect("Failed to parse command");
            let mut listener = reactor.event_listener().await;
            reactor.spawn_program(program).await;
            while let Some(event) = listener.recv().await {
                match event {
                    ReactorEvent::ProgramOutput {
                        content: ProgramOutput::Exit { message, .. },
                        id: 0,
                    } => {
                        if let Some(message) = message {
                            println!("{}", message);
                        }
                    }
                    ReactorEvent::ProgramOutput {
                        content: ProgramOutput::Text { message },
                        ..
                    } => {
                        println!("{}", message);
                    }
                    ReactorEvent::ProgramOutput {
                        content: ProgramOutput::Json { content },
                        ..
                    } => {
                        println!(
                            "{:?}",
                            &content
                        );
                    }
                    ReactorEvent::RuntimeCreated { .. } => {}
                    ReactorEvent::RuntimeDestroyed { id: 0 } => break,
                    e => {
                        dbg!(e);
                    }
                }
            }
        }
        None => println!("No subcommand was used"),
        _ => println!("Some other subcommand was used"),
    }
}
