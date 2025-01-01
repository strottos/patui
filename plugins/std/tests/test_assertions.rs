use std::{
    collections::HashMap,
    net::TcpListener,
    process::{Child, Command},
    time::Duration,
};

use assertor::*;
use escargot::CargoBuild;
use patui_core::{
    ptplugin::{plugin_service_client::PluginServiceClient, publish, run, shutdown, wait},
    PatuiData, PatuiDataInner, PatuiEvent, PatuiEventWithTimestamp,
};
use tokio::time::timeout;
use tonic::{transport::Channel, Request};

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

pub(crate) fn get_unused_localhost_port() -> Result<u16, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

#[tokio::test]
async fn simple_assertion() {
    let port = get_unused_localhost_port().unwrap();

    let mut child = run_plugin(port).await;

    let mut client = connect_plugin(port).await;

    let res = client
        .run(run::Request {
            function: "assertion".to_string(),
            args: HashMap::from([("expr".to_string(), "[1,2,3][1] == 2".to_string())]),
        })
        .await;

    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let response = response.unwrap();
    assert_that!(response.diagnostics).has_length(0);
    assert_that!(response.data).is_some();

    let event = PatuiEventWithTimestamp::try_from(response.data.unwrap());
    assert_that!(event).is_ok();
    let event = event.unwrap();
    assert_that!(event.value()).is_equal_to(&PatuiEvent::Results(
        "assertion".to_string(),
        PatuiData::Known(PatuiDataInner::Bool(true)),
    ));

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}

#[tokio::test]
async fn simple_assertion_failure() {
    let port = get_unused_localhost_port().unwrap();

    let mut child = run_plugin(port).await;

    let mut client = connect_plugin(port).await;

    let res = client
        .run(run::Request {
            function: "assertion".to_string(),
            args: HashMap::from([("expr".to_string(), "[1,2,3][1] == 3".to_string())]),
        })
        .await;

    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let response = response.unwrap();
    assert_that!(response.diagnostics).has_length(0);
    assert_that!(response.data).is_some();

    let event = PatuiEventWithTimestamp::try_from(response.data.unwrap());
    assert_that!(event).is_ok();
    let event = event.unwrap();
    assert_that!(event.value()).is_equal_to(&PatuiEvent::Failure(
        "assertion".to_string(),
        "evaluated expr to false: [1,2,3][1] == 3".to_string(),
    ));

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}

#[tokio::test]
async fn simple_assertion_error() {
    let port = get_unused_localhost_port().unwrap();

    let mut child = run_plugin(port).await;

    let mut client = connect_plugin(port).await;

    let res = client
        .run(run::Request {
            function: "assertion".to_string(),
            args: HashMap::from([("expr".to_string(), "[1,2,3][1]".to_string())]),
        })
        .await;

    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let response = response.unwrap();
    assert_that!(response.diagnostics).has_length(0);
    assert_that!(response.data).is_some();

    let event = PatuiEventWithTimestamp::try_from(response.data.unwrap());
    assert_that!(event).is_ok();
    let event = event.unwrap();
    assert_that!(event.value()).is_equal_to(&PatuiEvent::Error(
        "Assertion error, evaluated to type Integer: [1,2,3][1]".to_string(),
    ));

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}

#[tokio::test]
async fn assertion_with_idents_and_no_results() {
    let port = get_unused_localhost_port().unwrap();

    let mut child = run_plugin(port).await;

    let mut client = connect_plugin(port).await;

    let res = client
        .run(run::Request {
            function: "assertion".to_string(),
            args: HashMap::from([(
                "expr".to_string(),
                "my.foo.results.len() == 1 && my.foo.results[0] == 1".to_string(),
            )]),
        })
        .await;

    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let client_clone = client.clone();

    tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for data in [] {
                let data: PatuiData = data;
                tracing::trace!("Got data from receiver: {:?}", data);

                yield publish::Request {
                    function: "assertion".to_string(),
                    name: "my.foo.results".to_string(),
                    data: Some(data.try_into().unwrap()),
                }
            }
        };

        let mut response = client
            .publish(Request::new(outbound))
            .await
            .unwrap()
            .into_inner();

        let Ok(Some(resp)) = response.message().await else {
            panic!("No response");
        };
        tracing::trace!("RESP = {:?}", resp);
    });

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_err();
    let response = response.unwrap_err();
    assert_that!(response.code()).is_equal_to(tonic::Code::NotFound);

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}

#[tokio::test]
async fn assertion_with_idents_and_not_enough_results() {
    let port = get_unused_localhost_port().unwrap();

    let mut child = run_plugin(port).await;

    let mut client = connect_plugin(port).await;

    let res = client
        .run(run::Request {
            function: "assertion".to_string(),
            args: HashMap::from([(
                "expr".to_string(),
                "my.foo.results.len() == 1 && my.foo.results[3] == 1".to_string(),
            )]),
        })
        .await;

    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let client_clone = client.clone();

    tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for data in [PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                ("my".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    ("foo".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                        ("results".to_string(), PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(PatuiDataInner::Integer(1))]))),
                    ])))),
                ]))))
            ])))] {
                let data: PatuiData = data;
                tracing::trace!("Got data from receiver: {:?}", data);

                yield publish::Request {
                    function: "assertion".to_string(),
                    name: "my.foo.results".to_string(),
                    data: Some(data.try_into().unwrap()),
                }
            }
        };

        let mut response = client
            .publish(Request::new(outbound))
            .await
            .unwrap()
            .into_inner();

        let Ok(Some(resp)) = response.message().await else {
            panic!("No response");
        };
        tracing::trace!("RESP = {:?}", resp);
    });

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_err();
    let response = response.unwrap_err();
    assert_that!(response.code()).is_equal_to(tonic::Code::NotFound);

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}

#[tokio::test]
async fn assertion_with_idents_and_correct_results() {
    let port = get_unused_localhost_port().unwrap();

    let mut child = run_plugin(port).await;

    let mut client = connect_plugin(port).await;

    let res = client
        .run(run::Request {
            function: "assertion".to_string(),
            args: HashMap::from([(
                "expr".to_string(),
                "(my.foo.results.len() == 1) && (my.foo.results[0] == 1)".to_string(),
            )]),
        })
        .await;

    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let client_clone = client.clone();

    tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for data in [PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                ("my".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    ("foo".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                        ("results".to_string(), PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(PatuiDataInner::Integer(1))]))),
                    ])))),
                ]))))
            ])))] {
                let data: PatuiData = data;
                tracing::trace!("Got data from receiver: {:?}", data);

                yield publish::Request {
                    function: "assertion".to_string(),
                    name: "my.foo.results".to_string(),
                    data: Some(data.try_into().unwrap()),
                }
            }
        };

        let mut response = client
            .publish(Request::new(outbound))
            .await
            .unwrap()
            .into_inner();

        let Ok(Some(resp)) = response.message().await else {
            panic!("No response");
        };
        tracing::trace!("RESP = {:?}", resp);
    });

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let response = response.unwrap();
    assert_that!(response.diagnostics).has_length(0);
    assert_that!(response.data).is_some();

    let event = PatuiEventWithTimestamp::try_from(response.data.unwrap());
    assert_that!(event).is_ok();
    let event = event.unwrap();
    assert_that!(event.value()).is_equal_to(&PatuiEvent::Results(
        "assertion".to_string(),
        PatuiData::Known(PatuiDataInner::Bool(true)),
    ));

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}

#[tokio::test]
async fn assertion_with_idents_and_incorrect_results() {
    let port = get_unused_localhost_port().unwrap();

    let mut child = run_plugin(port).await;

    let mut client = connect_plugin(port).await;

    let res = client
        .run(run::Request {
            function: "assertion".to_string(),
            args: HashMap::from([(
                "expr".to_string(),
                "(my.foo.results.len() == 1) && (my.foo.results[0] == 1)".to_string(),
            )]),
        })
        .await;

    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let client_clone = client.clone();

    tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for data in [PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                ("my".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    ("foo".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                        ("results".to_string(), PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(PatuiDataInner::Integer(1))]))),
                    ])))),
                ])))),
                ("my".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    ("foo".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                        ("results".to_string(), PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(PatuiDataInner::Integer(1)), PatuiData::Known(PatuiDataInner::Integer(2))]))),
                    ])))),
                ]))))
            ])))] {
                let data: PatuiData = data;
                tracing::trace!("Got data from receiver: {:?}", data);

                yield publish::Request {
                    function: "assertion".to_string(),
                    name: "my.foo.results".to_string(),
                    data: Some(data.try_into().unwrap()),
                }
            }
        };

        let mut response = client
            .publish(Request::new(outbound))
            .await
            .unwrap()
            .into_inner();

        let Ok(Some(resp)) = response.message().await else {
            panic!("No response");
        };
        tracing::trace!("RESP = {:?}", resp);
    });

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let response = response.unwrap();
    assert_that!(response.diagnostics).has_length(0);
    assert_that!(response.data).is_some();

    let event = PatuiEventWithTimestamp::try_from(response.data.unwrap());
    assert_that!(event).is_ok();
    let event = event.unwrap();
    assert_that!(event.value()).is_equal_to(&PatuiEvent::Failure(
        "assertion".to_string(),
        "evaluated expr to false: (my.foo.results.len() == 1) && (my.foo.results[0] == 1)"
            .to_string(),
    ));

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}
