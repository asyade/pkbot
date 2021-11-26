use super::*;

pub async fn main(
    reactor: Reactor,
    mut args: Vec<String>,
    _stdin: Option<Receiver<ProgramOutput>>,
    stdout: Sender<ProgramOutput>,
) {
    args.insert(0, "cat".to_string());
    let app = clap::App::new("cat").arg(
        Arg::new("duration")
            .required(true)
            .takes_value(true)
            .multiple_values(true),
    );
    match app.try_get_matches_from(args) {
        Ok(app) => {
            tokio::time::sleep(Duration::from_secs(10)).await;
            buitlin_result!(stdout);
        }
        Err(e) => {
            buitlin_panic!(stdout, "{}", e);
        }
    }
}