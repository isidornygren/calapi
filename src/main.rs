use actix_web::{error, get, web, App, HttpServer, Responder, Result};
use chrono::{Datelike, Duration, Months, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use ics::components::Property;
use ics::{Event, ICalendar};
use reqwest::Client;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer,
};
use serde_derive::Deserialize;

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

#[derive(std::cmp::PartialEq, Debug)]
struct TimeSlotDesc {
    date: NaiveDateTime,
    duration: Duration,
}

#[derive(Debug)]
enum TimeSlotParseError {
    ParseMonth,
    ParseDay,
    ParseEnd,
    StartDate,
    EndDate,
    Overflow,
}

impl std::fmt::Display for TimeSlotParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseMonth => {
                write!(f, "Could not parse month")
            }
            Self::ParseDay => {
                write!(f, "Could not parse day")
            }
            Self::ParseEnd => {
                write!(f, "Could not parse end")
            }
            Self::StartDate => {
                write!(f, "Could not parse start date")
            }
            Self::EndDate => {
                write!(f, "Could not parse end date")
            }
            Self::Overflow => {
                write!(f, "Year overflow")
            }
        }
    }
}

fn deserialize_time_slot_desc(
    time_slot_desc: &str,
    start_date: NaiveDate,
) -> Result<TimeSlotDesc, TimeSlotParseError> {
    let values: Vec<&str> = time_slot_desc
        .split_ascii_whitespace()
        .filter(|w| *w != "-")
        .collect();

    let end_date = match values.get(2) {
        Some(date_str) => {
            let replaced_str = date_str.replace(&['(', ')'][..], "");
            let month_and_day: Vec<&str> = replaced_str.split('/').collect();

            let end_date = NaiveDate::from_ymd_opt(
                start_date.year(),
                month_and_day
                    .get(1)
                    .unwrap()
                    .parse::<u32>()
                    .map_err(|_| TimeSlotParseError::ParseMonth)?,
                month_and_day
                    .first()
                    .unwrap()
                    .parse::<u32>()
                    .map_err(|_| TimeSlotParseError::ParseDay)?,
            )
            .ok_or(TimeSlotParseError::ParseEnd)?;

            if end_date < start_date {
                end_date
                    .checked_add_months(Months::new(12))
                    .ok_or(TimeSlotParseError::Overflow)?
            } else {
                end_date
            }
        }
        None => start_date,
    };

    let start =
        NaiveTime::parse_from_str(values[0], "%H:%M").map_err(|_| TimeSlotParseError::StartDate)?;
    let end =
        NaiveTime::parse_from_str(values[1], "%H:%M").map_err(|_| TimeSlotParseError::EndDate)?;
    let start_date_time = NaiveDateTime::new(start_date, start);
    let end_date_time = NaiveDateTime::new(end_date, end);

    Ok(TimeSlotDesc {
        date: start_date_time,
        duration: end_date_time.signed_duration_since(start_date_time),
    })
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
        event.push(Property::new(
            "DTSTART",
            desc.date.format(EVENT_DT_FORMAT).to_string(),
        ));
        event.push(Property::new("SUMMARY", booking.laundry_room.clone()));
        event.push(Property::new("LOCATION", booking.laundry_room.clone()));
        event.push(Property::new(
            "DURATION",
            format!("PT{}H", desc.duration.num_hours()),
        ));
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};

    use crate::{deserialize_time_slot_desc, TimeSlotDesc};

    #[test]
    fn deserialize_time_slot_desc_parses_start_and_end() {
        let date = NaiveDate::parse_from_str("2022-08-22", "%Y-%m-%d").unwrap();
        let result = deserialize_time_slot_desc("15:00 - 16:00", date).unwrap();
        assert_eq!(
            result,
            TimeSlotDesc {
                duration: Duration::hours(1),
                date: NaiveDateTime::new(
                    date,
                    NaiveTime::parse_from_str("15:00", "%H:%M").unwrap()
                )
            }
        );
    }

    #[test]
    fn deserialize_time_slot_desc_parses_start_and_end_and_date() {
        let date = NaiveDate::parse_from_str("2022-08-22", "%Y-%m-%d").unwrap();
        let result = deserialize_time_slot_desc("15:00 - 16:00 (23/8)", date).unwrap();

        assert_eq!(
            result,
            TimeSlotDesc {
                duration: Duration::hours(25),
                date: NaiveDateTime::new(
                    date,
                    NaiveTime::parse_from_str("15:00", "%H:%M").unwrap()
                )
            }
        );
    }

    #[test]
    fn deserialize_time_slot_desc_parses_start_and_end_and_date_next_year() {
        let date = NaiveDate::parse_from_str("2022-12-31", "%Y-%m-%d").unwrap();
        let result = deserialize_time_slot_desc("15:00 - 16:00 (1/1)", date).unwrap();

        assert_eq!(
            result,
            TimeSlotDesc {
                duration: Duration::hours(25),
                date: NaiveDateTime::new(
                    date,
                    NaiveTime::parse_from_str("15:00", "%H:%M").unwrap()
                )
            }
        );
    }
}
