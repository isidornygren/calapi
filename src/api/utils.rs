use chrono::NaiveDate;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer,
};

pub fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
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

pub const API_DATE_FORMAT: &str = "%Y-%m-%d";

pub fn date_from_string<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&s, API_DATE_FORMAT).map_err(serde::de::Error::custom)
}
