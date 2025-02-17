use std::{collections::HashMap, time::Duration};

use assertor::*;
use ptplugin::{
    async_stream, check_event_response,
    plugin_server::{receive_results, run},
    run_plugin, shutdown_plugin,
    tokio::{self, time::timeout},
    tonic::Request,
    tracing, PatuiData, PatuiDataInner, PatuiEvent, PatuiStepResult, PatuiStepResultStatus,
};
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn unordered_test() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for result in [] {
                let result: PatuiStepResult = result;
                yield receive_results::Request {
                    result: Some(result.try_into().unwrap()),
                }
            }
        };

        client
            .receive_results(Request::new(outbound))
            .await
            .unwrap()
            .into_inner()
    });

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "unordered_list".to_string(),
            args: HashMap::from([]),
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    // Wait for the results to be sent to the plugin
    let ret = results_to_plugin_task.await;
    assert_that!(ret).is_ok();
    let ret = ret.unwrap();
    drop(ret);

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
}
