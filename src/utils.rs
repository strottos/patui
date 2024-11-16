use chrono::Local;

pub(crate) fn get_current_time_string() -> String {
    Local::now().to_string()
}

pub(crate) fn get_current_timestamp() -> i64 {
    Local::now().timestamp_millis()
}
