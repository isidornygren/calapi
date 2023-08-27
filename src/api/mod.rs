mod my_booking;
mod utils;

use serde_derive::Deserialize;

pub use my_booking::MyBooking;

#[derive(Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub error: i32,
    pub message: String,
    pub body: T,
    pub api_exec_time: f32,
}
