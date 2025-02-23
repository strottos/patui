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
async fn assert_static_truth() {
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
            function: "assertion".to_string(),
            args: HashMap::from([("expr".to_string(), "[1,2,3][1] == 2".to_string())]),
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    // Wait for the results to be sent to the plugin
    let send_results_task = tokio::spawn(async move {
        let ret = results_to_plugin_task.await;
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        drop(ret);
    });

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::set_item(
        "steps.bar.assertion.result".try_into().unwrap(),
        true.into(),
        PatuiData::Known(PatuiDataInner::Bool(true)),
    )));

    let task_result = timeout(Duration::from_secs(2), send_results_task).await;
    assert_that!(task_result).is_ok();
    let task_result = task_result.unwrap();
    assert_that!(task_result).is_ok();

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn assert_static_false() {
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
            function: "assertion".to_string(),
            args: HashMap::from([("expr".to_string(), "[1,2,3][1] == 3".to_string())]),
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    // Wait for the results to be sent to the plugin
    let send_results_task = tokio::spawn(async move {
        let ret = results_to_plugin_task.await;
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        drop(ret);
    });

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::set_item(
        "steps.bar.assertion.result".try_into().unwrap(),
        false.into(),
        PatuiData::Known(PatuiDataInner::Bool(false)),
    )));

    let task_result = timeout(Duration::from_secs(2), send_results_task).await;
    assert_that!(task_result).is_ok();
    let task_result = task_result.unwrap();
    assert_that!(task_result).is_ok();

    shutdown_plugin(child, client).await;
}
