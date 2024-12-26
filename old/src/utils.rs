use chrono::Local;
use eyre::Result;
use tokio::net::TcpListener;

pub(crate) fn get_current_time_string() -> String {
    Local::now().to_string()
}

pub(crate) fn get_current_timestamp() -> i64 {
    Local::now().timestamp_millis()
}

pub(crate) async fn get_unused_localhost_port() -> Result<u16> {
    let listener = TcpListener::bind(format!("127.0.0.1:0")).await?;
    Ok(listener.local_addr()?.port())
}
