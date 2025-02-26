use std::{collections::HashMap, time::Duration};

use assertor::*;
use ptplugin::{
    plugin_server::run,
    run_plugin, run_results_test_server, shutdown_plugin, shutdown_results_test_server,
    tokio::{self, sync::mpsc, time::timeout},
    tracing, PatuiData, PatuiDataInner, PatuiEvent, PatuiStepResult,
};
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn assert_static_truth() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, mut results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "assertion".to_string(),
            args: HashMap::from([("expr".to_string(), "[1,2,3][1] == 2".to_string())]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();

    let response = timeout(Duration::from_secs(2), results_receive_rx.recv()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let event = response.unwrap();
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::set_item(
        "steps.bar.assertion.result".try_into().unwrap(),
        true.into(),
        PatuiData::Known(PatuiDataInner::Bool(true)),
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn assert_static_false() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, mut results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "assertion".to_string(),
            args: HashMap::from([("expr".to_string(), "[1,2,3][1] == 3".to_string())]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();

    let response = timeout(Duration::from_secs(2), results_receive_rx.recv()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let event = response.unwrap();
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::set_item(
        "steps.bar.assertion.result".try_into().unwrap(),
        false.into(),
        PatuiData::Known(PatuiDataInner::Bool(false)),
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}
