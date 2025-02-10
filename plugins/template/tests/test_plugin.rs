use std::{collections::HashMap, time::Duration};

use assertor::*;
use ptplugin::{
    plugin_server::{receive_results, run, shutdown, ResultType},
    run_plugin, shutdown_plugin, PatuiData, PatuiDataInner, PatuiEvent, PatuiResultType,
};
use tokio::time::timeout;
use tonic::Request;
use tracing_test::traced_test;

async fn check_event_response(
    response: Result<Result<Option<run::Response>, tonic::Status>, tokio::time::error::Elapsed>,
) -> PatuiEvent {
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let response = response.unwrap();
    assert_that!(response.data).is_some();
    let data = response.data;
    assert_that!(data).is_some();
    let event = data.unwrap().try_into();
    assert_that!(event).is_ok();
    event.unwrap()
}

#[traced_test]
#[tokio::test]
async fn echo_once_static() {
    let (child, mut client) = run_plugin("patui-template").await.unwrap();

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
    assert_that!(event).is_equal_to(PatuiEvent::Results(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        PatuiResultType::Append,
        PatuiData::Known(PatuiDataInner::String("Hello, World!".to_string())),
    ));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done);

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn echo_multiple_static() {
    let (child, mut client) = run_plugin("patui-template").await.unwrap();

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
        assert_that!(event).is_equal_to(PatuiEvent::Results(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::Append,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done);

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn echo_once() {
    let (child, mut client) = run_plugin("patui-template").await.unwrap();

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
    assert_that!(event).is_equal_to(PatuiEvent::Results(
        "steps.bar.echo.out".try_into().unwrap(),
        true.into(),
        PatuiResultType::Append,
        PatuiData::Known(PatuiDataInner::String("Hello, world!".to_string())),
    ));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done);

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn echo_multiple() {
    let (child, mut client) = run_plugin("patui-template").await.unwrap();

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
        assert_that!(event).is_equal_to(PatuiEvent::Results(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::Append,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done);

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn produce_before_run() {
    let (child, mut client) = run_plugin("patui-template").await.unwrap();

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
        assert_that!(event).is_equal_to(PatuiEvent::Results(
            "steps.bar.echo.out".try_into().unwrap(),
            true.into(),
            PatuiResultType::Append,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done);

    shutdown_plugin(child, client).await;
}

#[traced_test]
#[tokio::test]
async fn subscribe_after_run() {}

#[traced_test]
#[tokio::test]
async fn much_sleeping() {}

#[traced_test]
#[tokio::test]
async fn extensive_wait() {}

// benches?
#[traced_test]
#[tokio::test]
async fn load_test() {}

#[traced_test]
#[tokio::test]
async fn send_results_after_done() {}
