use std::{
    net::TcpListener,
    process::{Child, Command},
};

use assertor::*;
use escargot::CargoBuild;
use tonic::transport::Channel;

use self::ptplugin::{plugin_service_client::PluginServiceClient, run, shutdown, wait};

pub mod ptplugin {
    tonic::include_proto!("ptplugin");
}

async fn run_plugin(port: u16) -> Child {
    let cli = CargoBuild::new()
        .bin("patui-std")
        .current_target()
        .manifest_path("Cargo.toml")
        .target_dir("./target/debug")
        .run()
        .unwrap();

    let mut cmd = Command::new(cli.path());
    cmd.args(["--port", &port.to_string()])
        .env("PATUI_LOG", "debug")
        .spawn()
        .unwrap()
}

async fn connect_plugin(port: u16) -> PluginServiceClient<Channel> {
    for i in 0..50 {
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

pub(crate) fn get_unused_localhost_port() -> Result<u16, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

#[tokio::test]
async fn test_simple_assertion() {
    let port = get_unused_localhost_port().unwrap();

    let mut child = run_plugin(port).await;

    let mut client = connect_plugin(port).await;

    let res = client
        .run(run::Request {
            function: "assertion".to_string(),
        })
        .await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}
