use actix_web::{error, get, web, App, HttpServer, Responder, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use ics::properties::{Action, Description, DtStart, Duration, Location, Status, Summary, Trigger};
use ics::{Alarm, Event, ICalendar, TimeZone};
use reqwest::Client;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer,
};
use serde_derive::Deserialize;

mod time_slot_desc;

use time_slot_desc::deserialize_time_slot_desc;

fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(de::Error::invalid_value(
            Unexpected::Unsigned(other as u64),
            &"zero or one",
        )),
    }
}

const FORMAT: &str = "%Y-%m-%d";

fn api_date_format<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Booking {
    #[serde(rename = "BookingID")]
    booking_id: u32,
    #[serde(rename = "LaundryRoomID")]
    laundry_room_id: u32,
    laundry_room: String,
    time_slots: u32,
    time_slots_desc: String,
    date_text: String,
    image: String,
    #[serde(deserialize_with = "bool_from_int")]
    is_reminder: bool,
    #[serde(deserialize_with = "api_date_format")]
    date_reminder: NaiveDate,
    #[serde(deserialize_with = "api_date_format")]
    date_book: NaiveDate,
    date_reminder_text: String,
    hour_reminder: String,
    #[serde(deserialize_with = "bool_from_int")]
    is_queue: bool,
    #[serde(deserialize_with = "bool_from_int")]
    number_queue: bool,
    number_queue_text: String,
}

#[derive(Deserialize, Debug)]
struct TimeType {
    id: u32,
    name: String,
    #[serde(deserialize_with = "bool_from_int")]
    selected: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Reminder {
    time_type: Vec<TimeType>,
}

#[derive(Deserialize, Debug)]
struct MyBooking {
    data: Vec<Booking>,
    total: u32,
    reminder: Option<Reminder>,
}

#[derive(Deserialize, Debug)]
struct ApiResponse<T> {
    error: i32,
    message: String,
    body: T,
    api_exec_time: f32,
}

const EVENT_DT_FORMAT: &str = "%Y%m%dT%H%M%S";

#[get("/calendar/{user_id}/calendar.ics")]
async fn calendar(user_id: web::Path<String>) -> Result<impl Responder> {
    let client = Client::new();

    let query = vec![
        ("method", "getMyBooking"),
        ("userId", &user_id),
        ("start", "0"),
        ("limit", "100"),
        ("lang", "0"),
    ];

    let request = client
        .get("http://prod.bokatvattid.se/api/api2")
        .query(&query);

    let bookings: ApiResponse<MyBooking> = request
        .send()
        .await
        .map_err(error::ErrorInternalServerError)?
        .json()
        .await
        .map_err(error::ErrorInternalServerError)?;

    let mut calendar = ICalendar::new("2.0", "bokatvattid-api");

    for booking in &bookings.body.data {
        let desc = deserialize_time_slot_desc(&booking.time_slots_desc, booking.date_book)
            .map_err(error::ErrorInternalServerError)?;
        let mut event = Event::new(
            booking.booking_id.to_string(),
            Utc::now().format(EVENT_DT_FORMAT).to_string(),
        );
        event.push(DtStart::new(desc.date.format(EVENT_DT_FORMAT).to_string()));
        event.push(Summary::new(&booking.laundry_room));
        event.push(Location::new(&booking.laundry_room));

        if booking.is_queue {
            event.push(Status::tentative());
        } else {
            event.push(Status::confirmed());
        }

        event.push(Duration::new(format!("PT{}H", desc.duration.num_hours())));

        // let alarm_date = NaiveDate::parse_from_str(&booking.date_reminder, "%H:%M");
        let alarm_date = NaiveDateTime::new(
            booking.date_reminder,
            NaiveTime::parse_from_str(&booking.hour_reminder, "%H:%M")
                .map_err(error::ErrorInternalServerError)?,
        );
        let alarm_hour_diff = (alarm_date - desc.date).num_hours();

        let mut alarm = Alarm::new(
            Action::display(),
            Trigger::new(format!(
                "{}PT{}H",
                if alarm_hour_diff.signum() < 0 {
                    "-"
                } else {
                    ""
                },
                alarm_hour_diff.abs()
            )),
        );
        alarm.push(Description::new(&booking.laundry_room));
        event.add_alarm(alarm);

        calendar.add_event(event);
    }

    Ok(calendar.to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(calendar))
        .bind(("0.0.0.0", 10000))?
        .run()
        .await
}
