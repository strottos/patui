use chrono::Local;
use thiserror::Error;
use tokio::net::TcpListener;

#[derive(Debug, Error)]
pub enum PortError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("AddrParse error: {0}")]
    AddrParseError(#[from] std::net::AddrParseError),
}

pub(crate) fn get_current_time_string() -> String {
    Local::now().to_string()
}

pub(crate) fn get_current_timestamp() -> i64 {
    Local::now().timestamp_millis()
}

pub(crate) async fn get_unused_localhost_port() -> Result<u16, PortError> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    Ok(listener.local_addr()?.port())
}
