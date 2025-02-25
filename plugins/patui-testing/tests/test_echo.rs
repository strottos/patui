use std::{collections::HashMap, time::Duration};

use assertor::*;
use ptplugin::{
    check_event_response, connect_plugin,
    plugin_server::{receive_results, run},
    run_plugin, run_results_test_server, shutdown_plugin, shutdown_results_test_server,
    spawn_plugin,
    tokio::{self, sync::mpsc, time::timeout},
    tonic::Request,
    tracing, PatuiData, PatuiDataInner, PatuiEvent, PatuiStepResult, PatuiStepResultStatus,
};
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn echo_once_static() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, mut results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "[\"Hello, World!\"]".to_string())]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        0,
        PatuiData::Known(PatuiDataInner::String("Hello, World!".to_string())),
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        1,
    )));

    let response = timeout(Duration::from_secs(2), results_receive_rx.recv()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let event = response.unwrap();
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        0,
        PatuiData::Known(PatuiDataInner::String("Hello, World!".to_string())),
    )));

    let response = timeout(Duration::from_secs(2), results_receive_rx.recv()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let event = response.unwrap();
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        1,
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn echo_multiple_static() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([(
                "in".to_string(),
                r#"["Hello, 1!", "Hello, 2!", "Hello, 3!", "Hello, 4!", "Hello, 5!"]"#.to_string(),
            )]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            i - 1,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        )));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        5,
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn echo_once() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        for (i, data) in [PatuiData::Known(PatuiDataInner::String(
            "Hello, world!".to_string(),
        ))]
        .into_iter()
        .enumerate()
        {
            let result = PatuiStepResult::new_stream_item(
                "steps.foo.bar.results".try_into().unwrap(),
                PatuiStepResultStatus::Success,
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
        }

        let result = PatuiStepResult::done_stream(
            "steps.foo.bar.results".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            1,
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
            function: "echo".to_string(),
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

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        0,
        PatuiData::Known(PatuiDataInner::String("Hello, world!".to_string())),
    )));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.echo.out".try_into().unwrap(),
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
async fn echo_multiple() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        for (i, data) in [
            PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
            PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
            PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
            PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
            PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
        ]
        .into_iter()
        .enumerate()
        {
            let result = PatuiStepResult::new_stream_item(
                "steps.foo.bar.results".try_into().unwrap(),
                PatuiStepResultStatus::Success,
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
            PatuiStepResultStatus::Success,
            5,
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
            function: "echo".to_string(),
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

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            i - 1,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        )));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        5,
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
async fn produce_before_run() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        for (i, data) in [
            PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
            PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
            PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
            PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
            PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
        ]
        .into_iter()
        .enumerate()
        {
            let result = PatuiStepResult::new_stream_item(
                "steps.foo.bar.results".try_into().unwrap(),
                PatuiStepResultStatus::Success,
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
            PatuiStepResultStatus::Success,
            5,
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(Request::new(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            }))
            .await
            .unwrap()
            .into_inner()
    });

    // Wait for the results to be sent to the plugin
    let ret = results_to_plugin_task.await;
    assert_that!(ret).is_ok();
    let ret = ret.unwrap();
    drop(ret);

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo.bar.results".to_string())]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            i - 1,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        )));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        5,
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn out_of_order_results() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        let result = PatuiStepResult::new_stream_item(
            "steps.foo.bar.results".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            0,
            PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(Request::new(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            }))
            .await
            .unwrap()
            .into_inner();

        let result = PatuiStepResult::done_stream(
            "steps.foo.bar.results".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            5,
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(Request::new(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            }))
            .await
            .unwrap()
            .into_inner();

        let result = PatuiStepResult::new_stream_item(
            "steps.foo.bar.results".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            4,
            PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(Request::new(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            }))
            .await
            .unwrap()
            .into_inner();

        let result = PatuiStepResult::new_stream_item(
            "steps.foo.bar.results".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            3,
            PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(Request::new(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            }))
            .await
            .unwrap()
            .into_inner();

        let result = PatuiStepResult::new_stream_item(
            "steps.foo.bar.results".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            2,
            PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
        );

        tracing::trace!("Sending results to plugin: {:?}", result);

        client
            .receive_results(Request::new(receive_results::Request {
                result: Some(result.try_into().unwrap()),
            }))
            .await
            .unwrap()
            .into_inner();

        let result = PatuiStepResult::new_stream_item(
            "steps.foo.bar.results".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            1,
            PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
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

    // Wait for the results to be sent to the plugin
    let ret = results_to_plugin_task.await;
    assert_that!(ret).is_ok();

    // Run the plugin
    let res = timeout(
        Duration::from_secs(1),
        client.run(run::Request {
            step_name: "bar".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo.bar.results".to_string())]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            i - 1,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        )));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        5,
    )));

    shutdown_plugin(child, client).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}

#[traced_test]
#[tokio::test]
async fn multiple_different_runs() {
    let (child, port) = spawn_plugin("patui-testing-plugin").await.unwrap();
    let mut client1 = connect_plugin(port).await;
    let mut client2 = connect_plugin(port).await;
    let mut client3 = connect_plugin(port).await;

    let (results_receive_tx, results_receive_rx) = mpsc::channel(16);

    let (result_server_address, result_server_task, result_server_shutdown_tx) =
        run_results_test_server(results_receive_tx).await.unwrap();

    // All get sent on first connection, doesn't matter who sends results, this isn't a typo, it's
    // to check that this is independent of who sends results.
    let client1_clone = client1.clone();
    let client2_clone = client1.clone();
    let client3_clone = client1.clone();

    // Send results to the plugin
    let mut results_to_plugin_tasks = vec![
        tokio::spawn(async move {
            let mut client = client1_clone;
            for (i, data) in [
                PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
            ]
            .into_iter()
            .enumerate()
            {
                let result = PatuiStepResult::new_stream_item(
                    "steps.foo1.bar.results".try_into().unwrap(),
                    PatuiStepResultStatus::Success,
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
            }

            let result = PatuiStepResult::done_stream(
                "steps.foo1.bar.results".try_into().unwrap(),
                PatuiStepResultStatus::Success,
                5,
            );

            tracing::trace!("Sending results to plugin: {:?}", result);

            client
                .receive_results(Request::new(receive_results::Request {
                    result: Some(result.try_into().unwrap()),
                }))
                .await
                .unwrap()
                .into_inner();
        }),
        tokio::spawn(async move {
            let mut client = client2_clone;
            for (i, data) in [
                PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
            ]
            .into_iter()
            .enumerate()
            {
                let result = PatuiStepResult::new_stream_item(
                    "steps.foo2.bar.results".try_into().unwrap(),
                    PatuiStepResultStatus::Success,
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
            }

            let result = PatuiStepResult::done_stream(
                "steps.foo2.bar.results".try_into().unwrap(),
                PatuiStepResultStatus::Success,
                5,
            );

            tracing::trace!("Sending results to plugin: {:?}", result);

            client
                .receive_results(Request::new(receive_results::Request {
                    result: Some(result.try_into().unwrap()),
                }))
                .await
                .unwrap()
                .into_inner();
        }),
        tokio::spawn(async move {
            let mut client = client3_clone;
            for (i, data) in [
                PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
            ]
            .into_iter()
            .enumerate()
            {
                let result = PatuiStepResult::new_stream_item(
                    "steps.foo3.bar.results".try_into().unwrap(),
                    PatuiStepResultStatus::Success,
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
            }

            let result = PatuiStepResult::done_stream(
                "steps.foo3.bar.results".try_into().unwrap(),
                PatuiStepResultStatus::Success,
                5,
            );

            tracing::trace!("Sending results to plugin: {:?}", result);

            client
                .receive_results(Request::new(receive_results::Request {
                    result: Some(result.try_into().unwrap()),
                }))
                .await
                .unwrap()
                .into_inner();
        }),
    ];

    // Run the plugins
    let res = timeout(
        Duration::from_secs(1),
        client1.run(run::Request {
            step_name: "bar1".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo1.bar.results".to_string())]),
            result_server_address: result_server_address.clone(),
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription1_rx = res.unwrap().into_inner();

    let res = timeout(
        Duration::from_secs(1),
        client2.run(run::Request {
            step_name: "bar2".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo2.bar.results".to_string())]),
            result_server_address: result_server_address.clone(),
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription2_rx = res.unwrap().into_inner();

    let res = timeout(
        Duration::from_secs(1),
        client3.run(run::Request {
            step_name: "bar3".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo3.bar.results".to_string())]),
            result_server_address,
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription3_rx = res.unwrap().into_inner();

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription1_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar1.echo.out".try_into().unwrap(),
            true.into(),
            i - 1,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        )));

        let response = timeout(Duration::from_secs(2), subscription2_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar2.echo.out".try_into().unwrap(),
            true.into(),
            i - 1,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        )));

        let response = timeout(Duration::from_secs(2), subscription3_rx.message()).await;
        let event = check_event_response(response).await;
        tracing::info!("Got event from plugin: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
            "steps.bar3.echo.out".try_into().unwrap(),
            true.into(),
            i - 1,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        )));
    }

    let response = timeout(Duration::from_secs(2), subscription1_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar1.echo.out".try_into().unwrap(),
        true.into(),
        5,
    )));

    let response = timeout(Duration::from_secs(2), subscription2_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar2.echo.out".try_into().unwrap(),
        true.into(),
        5,
    )));

    let response = timeout(Duration::from_secs(2), subscription3_rx.message()).await;
    let event = check_event_response(response).await;
    tracing::info!("Got event from plugin: {:?}", event);
    assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
        "steps.bar3.echo.out".try_into().unwrap(),
        true.into(),
        5,
    )));

    // Check the results to be sent to the plugin
    for task in results_to_plugin_tasks.drain(..) {
        let ret = task.await;
        assert_that!(ret).is_ok();
    }

    shutdown_plugin(child, client1).await;
    shutdown_results_test_server(result_server_task, result_server_shutdown_tx).await;
}
