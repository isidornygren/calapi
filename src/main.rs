use actix_web::{error, get, web, App, HttpResponse, HttpServer, Responder, Result};
use ics::ICalendar;
use reqwest::{Client, StatusCode};

mod api;
mod time_slot_desc;

use api::ApiResponse;

use crate::api::MyBooking;

const EVENT_DT_FORMAT: &str = "%Y%m%dT%H%M%S";

#[get("/calendar/{user_id}/calendar.ics")]
async fn calendar(user_id: web::Path<u32>) -> Result<impl Responder> {
    let client = Client::new();
    let id_str = user_id.to_string();

    let query = vec![
        ("method", "getMyBooking"),
        ("userId", &id_str),
        ("start", "0"),
        ("limit", "100"),
        ("lang", "0"),
    ];

    let request = client
        .get("http://prod.bokatvattid.se/api/api2")
        .query(&query);

    let calendar: ICalendar = request
        .send()
        .await
        .map_err(error::ErrorInternalServerError)?
        .json::<ApiResponse<MyBooking>>()
        .await
        .map_err(error::ErrorInternalServerError)
        .and_then(|response| {
            if response.error < 0 {
                return Err(error::ErrorInternalServerError(response.message));
            }
            Ok(response.body)
        })?
        .try_into()
        .map_err(error::ErrorInternalServerError)?;

    Ok(calendar.to_string())
}

#[get("/")]
async fn start_page() -> impl Responder {
    HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../public/index.html"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(start_page).service(calendar))
        .bind(("0.0.0.0", 10000))?
        .run()
        .await
}
