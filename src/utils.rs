pub(crate) fn get_current_time_string() -> String {
    chrono::Local::now().to_string()
}
