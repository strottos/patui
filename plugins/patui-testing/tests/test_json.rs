use std::{collections::HashMap, time::Duration};

use assertor::*;
use ptplugin::{
    check_event_response,
    plugin_server::{receive_results, run},
    run_plugin, run_results_test_server, shutdown_plugin, shutdown_results_test_server,
    tokio::{self, sync::mpsc, time::timeout},
    tonic::Request,
    tracing, PatuiData, PatuiDataInner, PatuiEvent, PatuiStepResult,
};
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn transform_expr_string() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "json".to_string(),
            args: HashMap::from([("in".to_string(), r#""{\"a\":1,\"b\":2}""#.to_string())]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    // Wait for the results to be sent to the plugin
    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.json.out".try_into().unwrap(),
        true.into(),
        0,
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([
            (
                "a".to_string(),
                PatuiData::Known(PatuiDataInner::Integer(1)),
            ),
            (
                "b".to_string(),
                PatuiData::Known(PatuiDataInner::Integer(2)),
            ),
        ]))),
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.json.out".try_into().unwrap(),
        true.into(),
        1,
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn transform_multiple_expr_strings() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "json".to_string(),
            args: HashMap::from([(
                "in".to_string(),
                r#"["{\"a\":1}", "{\"a\":2}", "{\"a\":3}"]"#.to_string(),
            )]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    for (i, expected_data) in [
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(1)),
        )]))),
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(2)),
        )]))),
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(3)),
        )]))),
    ]
    .into_iter()
    .enumerate()
    {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar.json.out".try_into().unwrap(),
            true.into(),
            i,
            expected_data,
        )));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.json.out".try_into().unwrap(),
        true.into(),
        3,
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn transform_single_result() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        for (i, data) in [PatuiData::Known(PatuiDataInner::String(
            r#"{"a":1}"#.to_string(),
        ))]
        .into_iter()
        .enumerate()
        {
            let result = PatuiStepResult::new_stream_item(
                "steps.foo.bar.results".try_into().unwrap(),
                true.into(),
                i,
                data,
            );

            tracing::trace!("Sending results to plugin: {:?}", result);

            client
                .receive_results(receive_results::Request {
                    result: Some(result.try_into().unwrap()),
                })
                .await
                .unwrap()
                .into_inner();
        }

        let result = PatuiStepResult::done_stream(
            "steps.foo.bar.results".try_into().unwrap(),
            true.into(),
            1,
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            })
            .await
            .unwrap()
            .into_inner()
    });

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "json".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo.bar.results".to_string())]),
            result_server_address,
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
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.json.out".try_into().unwrap(),
        true.into(),
        0,
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(1)),
        )]))),
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.json.out".try_into().unwrap(),
        true.into(),
        1,
    )));

    let task_result = timeout(Duration::from_secs(2), send_results_task).await;
    assert_that!(task_result).is_ok();
    let task_result = task_result.unwrap();
    assert_that!(task_result).is_ok();

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn transform_multiple_result() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        for (i, data) in [
            PatuiData::Known(PatuiDataInner::String(r#"{"a":1}"#.to_string())),
            PatuiData::Known(PatuiDataInner::String(r#"{"a":2}"#.to_string())),
            PatuiData::Known(PatuiDataInner::String(r#"{"a":3}"#.to_string())),
        ]
        .into_iter()
        .enumerate()
        {
            let result = PatuiStepResult::new_stream_item(
                "steps.foo.bar.results".try_into().unwrap(),
                true.into(),
                i,
                data,
            );

            tracing::trace!("Sending results to plugin: {:?}", result);

            client
                .receive_results(receive_results::Request {
                    result: Some(result.try_into().unwrap()),
                })
                .await
                .unwrap()
                .into_inner();
        }

        let result = PatuiStepResult::done_stream(
            "steps.foo.bar.results".try_into().unwrap(),
            true.into(),
            3,
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            })
            .await
            .unwrap()
            .into_inner();
    });

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "json".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo.bar.results".to_string())]),
            result_server_address,
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
    });

    for (i, expected_data) in [
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(1)),
        )]))),
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(2)),
        )]))),
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(3)),
        )]))),
    ]
    .into_iter()
    .enumerate()
    {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar.json.out".try_into().unwrap(),
            true.into(),
            i,
            expected_data,
        )));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.json.out".try_into().unwrap(),
        true.into(),
        3,
    )));

    let task_result = timeout(Duration::from_secs(2), send_results_task).await;
    assert_that!(task_result).is_ok();
    let task_result = task_result.unwrap();
    assert_that!(task_result).is_ok();

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn transform_multiple_result_with_waits() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        for (i, data) in [
            PatuiData::Known(PatuiDataInner::String(r#"{"a":1}"#.to_string())),
            PatuiData::Known(PatuiDataInner::String(r#"{"a":2}"#.to_string())),
            PatuiData::Known(PatuiDataInner::String(r#"{"a":3}"#.to_string())),
        ]
        .into_iter()
        .enumerate()
        {
            let result = PatuiStepResult::new_stream_item(
                "steps.foo.bar.results".try_into().unwrap(),
                true.into(),
                i,
                data,
            );

            tracing::trace!("Sending results to plugin: {:?}", result);

            client
                .receive_results(Request::new(receive_results::Request {
                    result: Some(result.try_into().unwrap()),
                }))
                .await
                .unwrap()
                .into_inner();

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        let result = PatuiStepResult::done_stream(
            "steps.foo.bar.results".try_into().unwrap(),
            true.into(),
            3,
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(Request::new(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            }))
            .await
            .unwrap()
            .into_inner();
    });

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "json".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo.bar.results".to_string())]),
            result_server_address,
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
    });

    for (i, expected_data) in [
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(1)),
        )]))),
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(2)),
        )]))),
        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "a".to_string(),
            PatuiData::Known(PatuiDataInner::Integer(3)),
        )]))),
    ]
    .into_iter()
    .enumerate()
    {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        tracing::info!("Got response plugin: {:?}", response);
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar.json.out".try_into().unwrap(),
            true.into(),
            i,
            expected_data,
        )));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response plugin: {:?}", response);
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.json.out".try_into().unwrap(),
        true.into(),
        3,
    )));

    let task_result = timeout(Duration::from_secs(2), send_results_task).await;
    assert_that!(task_result).is_ok();
    let task_result = task_result.unwrap();
    assert_that!(task_result).is_ok();

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}
