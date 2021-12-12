use super::*;

pub async fn main(
    _reactor: Reactor,
    mut args: Vec<String>,
    _stdin: Option<Receiver<ProgramOutput>>,
    _stdout: Sender<ProgramOutput>,
) -> Result<ProgramOutput> {
    args.insert(0, "sleep".to_string());
    let app = clap::App::new("sleep").arg(
        Arg::new("duration")
            .required(true)
            .index(1)
            .takes_value(true)
            .multiple_values(true),
    );
    let app = app.try_get_matches_from(args)?;
    tokio::time::sleep(Duration::from_secs(
        app.value_of("duration").unwrap().parse().unwrap_or(0),
    ))
    .await;
    Ok(ProgramOutput::Exit {
        message: None,
        status: ProgramStatus::Success,
    })
}
