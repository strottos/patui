use std::{collections::HashMap, time::Duration};

use assertor::*;
use ptplugin::{
    check_event_response,
    plugin_server::run,
    run_plugin, run_results_test_server, shutdown_plugin, shutdown_results_test_server,
    tokio::{self, sync::mpsc, time::timeout},
    tracing, PatuiData, PatuiDataInner, PatuiEvent, PatuiStepResult,
};
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn unordered_test() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "unordered_list".to_string(),
            args: HashMap::from([]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.unordered_list.out".try_into().unwrap(),
        true.into(),
        0,
        PatuiData::Known(PatuiDataInner::Integer(0)),
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.unordered_list.out".try_into().unwrap(),
        true.into(),
        2,
        PatuiData::Known(PatuiDataInner::Integer(2)),
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.unordered_list.out".try_into().unwrap(),
        true.into(),
        4,
        PatuiData::Known(PatuiDataInner::Integer(4)),
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.unordered_list.out".try_into().unwrap(),
        true.into(),
        5,
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.unordered_list.out".try_into().unwrap(),
        true.into(),
        3,
        PatuiData::Known(PatuiDataInner::Integer(3)),
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.unordered_list.out".try_into().unwrap(),
        true.into(),
        1,
        PatuiData::Known(PatuiDataInner::Integer(1)),
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}
