use crate::{
    time_slot_desc::{deserialize_time_slot_desc, TimeSlotParseError},
    EVENT_DT_FORMAT,
};

use super::utils::{bool_from_int, date_from_string};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use ics::{
    properties::{Action, Description, DtStart, Duration, Location, Status, Summary, Trigger},
    Alarm, Event, ICalendar,
};
use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Booking {
    #[serde(rename = "BookingID")]
    pub booking_id: u32,
    #[serde(rename = "LaundryRoomID")]
    pub laundry_room_id: u32,
    pub laundry_room: String,
    pub time_slots: u32,
    pub time_slots_desc: String,
    pub date_text: String,
    pub image: String,
    #[serde(deserialize_with = "bool_from_int")]
    pub is_reminder: bool,
    #[serde(deserialize_with = "date_from_string")]
    pub date_reminder: NaiveDate,
    #[serde(deserialize_with = "date_from_string")]
    pub date_book: NaiveDate,
    pub date_reminder_text: String,
    pub hour_reminder: String,
    #[serde(deserialize_with = "bool_from_int")]
    pub is_queue: bool,
    pub number_queue: u32,
    pub number_queue_text: String,
}

#[derive(Deserialize, Debug)]
pub struct TimeType {
    pub id: u32,
    pub name: String,
    #[serde(deserialize_with = "bool_from_int")]
    pub selected: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Reminder {
    pub time_type: Vec<TimeType>,
}

#[derive(Deserialize, Debug)]
pub struct MyBooking {
    pub data: Vec<Booking>,
    pub total: u32,
    pub reminder: Option<Reminder>,
}

#[cfg(test)]
fn current_time() -> DateTime<Utc> {
    let date = NaiveDate::from_ymd_opt(2012, 11, 11)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    DateTime::<Utc>::from_utc(date, Utc)
}

#[cfg(not(test))]
fn current_time() -> DateTime<Utc> {
    Utc::now()
}

impl<'a> TryInto<ICalendar<'a>> for MyBooking {
    type Error = TimeSlotParseError;

    fn try_into(self) -> Result<ICalendar<'a>, Self::Error> {
        let mut calendar = ICalendar::new("2.0", "bokatvattid-api");

        for booking in self.data.into_iter() {
            let desc = deserialize_time_slot_desc(&booking.time_slots_desc, booking.date_book)?;
            let mut event = Event::new(
                booking.booking_id.to_string(),
                current_time().format(EVENT_DT_FORMAT).to_string(),
            );
            event.push(DtStart::new(desc.date.format(EVENT_DT_FORMAT).to_string()));
            event.push(Summary::new(booking.laundry_room.clone()));
            event.push(Location::new(booking.laundry_room.clone()));

            if booking.is_queue {
                event.push(Status::tentative());
            } else {
                event.push(Status::confirmed());
            }

            event.push(Duration::new(format!("PT{}H", desc.duration.num_hours())));

            let alarm_date = NaiveDateTime::new(
                booking.date_reminder,
                NaiveTime::parse_from_str(&booking.hour_reminder, "%H:%M")
                    .map_err(|_| TimeSlotParseError::AlarmDate)?,
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
            alarm.push(Description::new(booking.laundry_room));
            event.add_alarm(alarm);

            calendar.add_event(event);
        }
        Ok(calendar)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use ics::ICalendar;

    use crate::api::utils::API_DATE_FORMAT;

    use super::{Booking, MyBooking};

    #[test]
    fn my_booking_should_parse_into_calendar() {
        let my_booking: ICalendar = MyBooking {
            data: vec![Booking {
                booking_id: 1,
                laundry_room_id: 1,
                laundry_room: "Laundry room 1".to_string(),
                time_slots: 1,
                time_slots_desc: "10:00 - 11:00".to_string(),
                date_text: "".to_string(),
                image: "".to_string(),
                is_reminder: false,
                date_reminder: NaiveDate::parse_from_str("2012-11-11", API_DATE_FORMAT).unwrap(),
                date_book: NaiveDate::parse_from_str("2012-11-12", API_DATE_FORMAT).unwrap(),
                date_reminder_text: "Remember your booking!".to_string(),
                hour_reminder: "09:00".to_string(),
                is_queue: false,
                number_queue: 0,
                number_queue_text: "".to_string(),
            }],
            total: 1,
            reminder: None,
        }
        .try_into()
        .unwrap();

        assert_eq!(my_booking.to_string(), "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:bokatvattid-api\r\nBEGIN:VEVENT\r\nUID:1\r\nDTSTAMP:20121111T080000\r\nDTSTART:20121112T100000\r\nSUMMARY:Laundry room 1\r\nLOCATION:Laundry room 1\r\nSTATUS:CONFIRMED\r\nDURATION:PT1H\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT25H\r\nDESCRIPTION:Laundry room 1\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n")
    }
}
