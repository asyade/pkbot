use crate::prelude::*;

mod market;

impl<'r> rocket::response::Responder<'r, 'static> for crate::error::Error {
    fn respond_to(self, r: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        error!(
            "An error occured {} {:?} {:?} {:?}",
            self,
            r.method(),
            r.uri().path(),
            r.uri().query(),
        );
        rocket::response::Result::Ok(
            rocket::response::status::Custom(
                rocket::http::Status::InternalServerError,
                rocket::serde::json::Json(format!("{}", self)),
            )
            .respond_to(r)?,
        )
    }
}

pub async fn spawn(reactor: Reactor) -> Result<()> {
    rocket::build()
        .manage(reactor)
        .mount(
            "/market",
            routes![
                market::get,
                market::get_all,
                market::post,
                market::delete,
                market::data::get
            ],
        )
        .launch()
        .await?;
    Ok(())
}
