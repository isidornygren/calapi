use chrono::{Datelike, Duration, Months, NaiveDate, NaiveDateTime, NaiveTime};

#[derive(std::cmp::PartialEq, Debug)]
pub struct TimeSlotDesc {
    pub date: NaiveDateTime,
    pub duration: Duration,
}

#[derive(Debug)]
pub enum TimeSlotParseError {
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

pub fn deserialize_time_slot_desc(
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};

    use super::{deserialize_time_slot_desc, TimeSlotDesc};

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
