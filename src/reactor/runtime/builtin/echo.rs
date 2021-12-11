use super::*;

pub async fn main(
    _reactor: Reactor,
    mut args: Vec<String>,
    _stdin: Option<Receiver<ProgramOutput>>,
    stdout: Sender<ProgramOutput>,
) -> Result<ProgramOutput> {
    args.insert(0, "echo".to_string());
    let app = clap::App::new("echo").arg(
        Arg::new("message")
            .required(true)
            .takes_value(true)
            .multiple_values(true),
    );
    let app = app.try_get_matches_from(args)?;
    for message in app.values_of("message").unwrap() {
        buitlin_print!(stdout, "{}", message);
    }
    Ok(ProgramOutput::Exit {
        message: None,
        status: ProgramStatus::Success
    })
}
