use crate::prelude::*;

mod cors;
mod market;
use cors::CORS;

pub async fn spawn(reactor: Reactor) -> Result<()> {
    rocket::build()
        .manage(reactor)
        .attach(CORS)
        .mount(
            "/market",
            routes![market::get, market::get_all, market::get_ohlc,],
        )
        .launch()
        .await?;
    Ok(())
}
