use std::{
    env,
    net::TcpListener,
    process::{Child, Command},
};

use escargot::CargoBuild;
use eyre::Result;
use patui_core::ptplugin::plugin_service_client::PluginServiceClient;
use tonic::transport::Channel;

pub async fn run_plugin(bin_name: &str) -> Result<(Child, PluginServiceClient<Channel>)> {
    let port = get_unused_localhost_port()?;

    let cli = CargoBuild::new()
        .bin(bin_name)
        .current_target()
        .manifest_path("Cargo.toml")
        .target_dir("./target/debug")
        .run()
        .unwrap();

    let mut cmd = Command::new(cli.path());
    let child = cmd
        .args(["--port", &port.to_string()])
        .env("PATUI_LOG", env::var("PATUI_LOG").unwrap_or("".to_string()))
        .spawn()
        .unwrap();

    let client = connect_plugin(port).await;

    Ok((child, client))
}

async fn connect_plugin(port: u16) -> PluginServiceClient<Channel> {
    for _ in 0..50 {
        let addr = format!("http://[::1]:{}", port);
        let client = PluginServiceClient::connect(addr).await;
        match client {
            Ok(c) => return c,
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }

    panic!("Failed to connect to the plugin");
}

fn get_unused_localhost_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}
