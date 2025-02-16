use std::{collections::HashMap, time::Duration};

use assertor::*;
use ptplugin::{
    async_stream, check_event_response, connect_plugin,
    plugin_server::{receive_results, run, ResultType},
    run_plugin, shutdown_plugin, spawn_plugin,
    tokio::{self, time::timeout},
    tonic::Request,
    tracing, PatuiData, PatuiDataInner, PatuiEvent, PatuiResultType, PatuiResultTypeConfirm,
};
use tracing_test::traced_test;

#[traced_test]
#[tokio::test]
async fn echo_once_static() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for results in [] {
                let results: PatuiData = results;
                yield receive_results::Request {
                    step_name: "foo".to_string(),
                    function_name: "bar".to_string(),
                    result_name: "results".to_string(),
                    r#type: ResultType::Append.into(),
                    results: Some(results.try_into().unwrap()),
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
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "[\"Hello, World!\"]".to_string())]),
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
    assert_that!(event).is_equal_to(PatuiEvent::Result(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        PatuiResultType::List(0),
        PatuiData::Known(PatuiDataInner::String("Hello, World!".to_string())),
    ));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(1)));

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn echo_multiple_static() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for results in [] {
                let results: PatuiData = results;
                yield receive_results::Request {
                    step_name: "foo".to_string(),
                    function_name: "bar".to_string(),
                    result_name: "results".to_string(),
                    r#type: ResultType::Append.into(),
                    results: Some(results.try_into().unwrap()),
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
            function: "echo".to_string(),
            args: HashMap::from([(
                "in".to_string(),
                r#"["Hello, 1!", "Hello, 2!", "Hello, 3!", "Hello, 4!", "Hello, 5!"]"#.to_string(),
            )]),
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

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        tracing::info!("Got response from plugin: {:?}", response);
        let event = check_event_response(response).await;
        assert_that!(event).is_equal_to(PatuiEvent::Result(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::List(i - 1),
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(5)));

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn echo_once() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for results in [
                PatuiData::Known(PatuiDataInner::String("Hello, world!".to_string())),
            ] {
                tracing::trace!("Sending results to plugin: {:?}", results);

                yield receive_results::Request {
                    step_name: "foo".to_string(),
                    function_name: "bar".to_string(),
                    result_name: "results".to_string(),
                    r#type: ResultType::Append.into(),
                    results: Some(results.try_into().unwrap()),
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
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo.bar.results".to_string())]),
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
    assert_that!(event).is_equal_to(PatuiEvent::Result(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        PatuiResultType::List(0),
        PatuiData::Known(PatuiDataInner::String("Hello, world!".to_string())),
    ));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(1)));

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn echo_multiple() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for results in [
                PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
            ] {
                tracing::trace!("Sending results to plugin: {:?}", results);

                yield receive_results::Request {
                    step_name: "foo".to_string(),
                    function_name: "bar".to_string(),
                    result_name: "results".to_string(),
                    r#type: ResultType::Append.into(),
                    results: Some(results.try_into().unwrap()),
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
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo.bar.results".to_string())]),
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

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        tracing::info!("Got response from plugin: {:?}", response);
        let event = check_event_response(response).await;
        assert_that!(event).is_equal_to(PatuiEvent::Result(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::List(i - 1),
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(5)));

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn produce_before_run() {
    let (child, mut client) = run_plugin("patui-testing-plugin").await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for results in [
                PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
            ] {
                tracing::trace!("Sending results to plugin: {:?}", results);

                yield receive_results::Request {
                    step_name: "foo".to_string(),
                    function_name: "bar".to_string(),
                    result_name: "results".to_string(),
                    r#type: ResultType::Append.into(),
                    results: Some(results.try_into().unwrap()),
                }
            }
        };

        client
            .receive_results(Request::new(outbound))
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
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
        tracing::info!("Got response from plugin: {:?}", response);
        let event = check_event_response(response).await;
        assert_that!(event).is_equal_to(PatuiEvent::Result(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::List(i - 1),
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(5)));

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn multiple_different_runs() {
    let (child, port) = spawn_plugin("patui-testing-plugin").await.unwrap();
    let mut client1 = connect_plugin(port).await;
    let mut client2 = connect_plugin(port).await;
    let mut client3 = connect_plugin(port).await;

    // All get sent on first connection, doesn't matter who sends results, this isn't a typo, it's
    // to check that this is independent of who sends results.
    let client1_clone = client1.clone();
    let client2_clone = client1.clone();
    let client3_clone = client1.clone();

    // Send results to the plugin
    let mut results_to_plugin_tasks = vec![
        tokio::spawn(async move {
            let mut client = client1_clone;
            let outbound = async_stream::stream! {
                for results in [
                    PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
                ] {
                    tracing::trace!("Sending results to plugin: {:?}", results);

                    yield receive_results::Request {
                        step_name: "foo1".to_string(),
                        function_name: "bar".to_string(),
                        result_name: "results".to_string(),
                        r#type: ResultType::Append.into(),
                        results: Some(results.try_into().unwrap()),
                    }
                }
            };

            client
                .receive_results(Request::new(outbound))
                .await
                .unwrap()
                .into_inner()
        }),
        tokio::spawn(async move {
            let mut client = client2_clone;
            let outbound = async_stream::stream! {
                for results in [
                    PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
                ] {
                    tracing::trace!("Sending results to plugin: {:?}", results);

                    yield receive_results::Request {
                        step_name: "foo2".to_string(),
                        function_name: "bar".to_string(),
                        result_name: "results".to_string(),
                        r#type: ResultType::Append.into(),
                        results: Some(results.try_into().unwrap()),
                    }
                }
            };

            client
                .receive_results(Request::new(outbound))
                .await
                .unwrap()
                .into_inner()
        }),
        tokio::spawn(async move {
            let mut client = client3_clone;
            let outbound = async_stream::stream! {
                for results in [
                    PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                    PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
                ] {
                    tracing::trace!("Sending results to plugin: {:?}", results);

                    yield receive_results::Request {
                        step_name: "foo3".to_string(),
                        function_name: "bar".to_string(),
                        result_name: "results".to_string(),
                        r#type: ResultType::Append.into(),
                        results: Some(results.try_into().unwrap()),
                    }
                }
            };

            client
                .receive_results(Request::new(outbound))
                .await
                .unwrap()
                .into_inner()
        }),
    ];

    // Run the plugins
    let res = timeout(
        Duration::from_secs(1),
        client1.run(run::Request {
            step_name: "bar1".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo1.bar.results".to_string())]),
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
        }),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription3_rx = res.unwrap().into_inner();

    // Wait for the results to be sent to the plugin
    for task in results_to_plugin_tasks.drain(..) {
        let ret = task.await;
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        drop(ret);
    }

    for i in 1..6 {
        let response = timeout(Duration::from_secs(2), subscription1_rx.message()).await;
        tracing::info!("Got response from plugin: {:?}", response);
        let event = check_event_response(response).await;
        assert_that!(event).is_equal_to(PatuiEvent::Result(
            "steps.bar1.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::List(i - 1),
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));

        let response = timeout(Duration::from_secs(2), subscription2_rx.message()).await;
        tracing::info!("Got response from plugin: {:?}", response);
        let event = check_event_response(response).await;
        assert_that!(event).is_equal_to(PatuiEvent::Result(
            "steps.bar2.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::List(i - 1),
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));

        let response = timeout(Duration::from_secs(2), subscription3_rx.message()).await;
        tracing::info!("Got response from plugin: {:?}", response);
        let event = check_event_response(response).await;
        assert_that!(event).is_equal_to(PatuiEvent::Result(
            "steps.bar3.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::List(i - 1),
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));
    }

    let response = timeout(Duration::from_secs(2), subscription1_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(5)));

    let response = timeout(Duration::from_secs(2), subscription2_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(5)));

    let response = timeout(Duration::from_secs(2), subscription3_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(5)));

    shutdown_plugin(child, client1).await;
}
