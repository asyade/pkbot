use clap::{App, Arg, ArgMatches};

pub(crate) mod error;
pub(crate) mod prelude;

pub(crate) mod exchange;
pub(crate) mod reactor;
pub(crate) mod store;

mod api;

use exchange::*;
use prelude::*;
use reactor::*;
use tokio::join;

fn test_peakfinder(store: StoreMarketHandle, begin: i64, end: i64) {
    let peaks = reactor::utils::PeakFinder::new(store, (begin, end)).unwrap();

    let mut data = vec![];

    let mut offset = begin;
    while let Ok(Some(elem)) = peaks.dataset.ohlc(offset) {
        if offset > end {
            break;
        }
        data.push((
            elem.time,
            elem.open_normalized as f32,
            elem.high_normalized as f32,
            elem.low_normalized as f32,
            elem.close_normalized as f32,
        ));
        if let Ok(Some(next)) = peaks.dataset.next_ohlc(offset) {
            offset = next.time;
        } else {
            break;
        }
    }
}

async fn simulate_commad(matchs: &ArgMatches) -> Result<()> {
    // let store_path = std::env::var("STORE_PATH").expect("STORE_PATH");
    // let store = Store::new(PathBuf::from(store_path)).unwrap();
    // let mut store_handle = store.handle().market("kraken", "BTC-USD").unwrap();

    // let begin: i64 = matchs
    //     .value_of("begin")
    //     .unwrap()
    //     .parse()
    //     .expect("begin: timestamp");
    // let end: i64 = matchs
    //     .value_of("end")
    //     .unwrap()
    //     .parse()
    //     .expect("ebd: timestamp");

    // let begin_close = store_handle.close_to(begin).ok().flatten().expect("Metric");
    // let end_close = store_handle.close_to(end).ok().flatten().expect("Metric");
    // log::info!(
    //     "Begin simulation: REQUEST_RANGE={}..{} CLOSEST_RANGE={}..{}",
    //     begin,
    //     end,
    //     begin_close,
    //     end_close
    // );
    // let peak = test_peakfinder(store_handle, begin_close, end_close);
    // // dbg!(peak);
    Ok(())
}

#[rocket::main]
async fn main() {
    pretty_env_logger::init();

    let matches = App::new("pkbot")
        .author("Asya C.")
        .version("0.1")
        .subcommand(App::new("daemon").about("Launch a reactor deamon"))
        .subcommand(
            App::new("simulate")
                .about("Launch a reactor deamon")
                .arg(Arg::new("begin").index(1).required(true))
                .arg(Arg::new("end").index(2).required(true)),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("daemon") => {
            let store_path = std::env::var("STORE_PATH").expect("STORE_PATH");
            let store = Store::new(PathBuf::from(store_path)).unwrap();
            let reactor = Reactor::new(store.handle());
            api::spawn(reactor)
                .await
                .expect("Failed to launch api server");
        }
        Some("simulate") => {
            let matches = matches.subcommand_matches("simulate").unwrap();
            simulate_commad(matches).await.unwrap();
        }
        None => println!("No subcommand was used"),
        _ => println!("Some other subcommand was used"),
    }
}
