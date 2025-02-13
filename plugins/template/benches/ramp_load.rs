use std::{collections::HashMap, time::Duration};

use assertor::*;
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::time::timeout;
use tonic::Request;

use ptplugin::{
    connect_plugin,
    plugin_server::{plugin_service_client::PluginServiceClient, receive_results, run, ResultType},
    shutdown_plugin, spawn_plugin, PatuiData, PatuiDataInner, PatuiEvent, PatuiResultType,
};
use uuid::Uuid;

async fn test_plugin(mut client: PluginServiceClient<tonic::transport::Channel>, sleep: u64) {
    let uuid = Uuid::new_v4().to_string().replace("-", "_");

    let client_clone = client.clone();

    let uuid_clone = uuid.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let uuid = uuid_clone;
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            let uuid = uuid;
            for results in [
                PatuiData::Known(PatuiDataInner::String("Hello, 1!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 2!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 3!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 4!".to_string())),
                PatuiData::Known(PatuiDataInner::String("Hello, 5!".to_string())),
            ] {
                tracing::trace!("Sending results to plugin: {:?}", results);

                yield receive_results::Request {
                    step_name: format!("foo_{}", uuid),
                    function_name: "bar".to_string(),
                    result_name: "results".to_string(),
                    r#type: ResultType::Append.into(),
                    results: Some(results.try_into().unwrap()),
                };

                tokio::time::sleep(Duration::from_millis(sleep)).await;
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
            step_name: format!("bar_{}", uuid),
            function: "echo".to_string(),
            args: HashMap::from([("in".to_string(), format!("steps.foo_{}.bar.results", uuid))]),
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
        let event = event.unwrap();
        assert_that!(event).is_equal_to(PatuiEvent::Results(
            format!("steps.bar_{}.echo.out", uuid).try_into().unwrap(),
            true.into(),
            PatuiResultType::Append,
            PatuiData::Known(PatuiDataInner::String(format!("Hello, {}!", i))),
        ));
    }

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
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
    let event = event.unwrap();
    assert_that!(event).is_equal_to(PatuiEvent::Done);
}

fn bench_ramp_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("ramp_load");
    group.sample_size(10);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_time()
        .enable_io()
        .build()
        .unwrap();

    let (child, client, port) = rt.block_on(async move {
        let (child, port) = spawn_plugin("patui-template").await.unwrap();
        let client = connect_plugin(port).await;
        (child, client, port)
    });

    eprintln!("Setup patui-template: {:?}", child);

    group.bench_function("ramp_load_1", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let client = connect_plugin(port).await;
                test_plugin(client, 0).await;
            });
        })
    });

    group.finish();

    rt.block_on(async move {
        shutdown_plugin(child, client).await;
    });
}

fn bench_ramp_load_sleep(c: &mut Criterion) {
    let mut group = c.benchmark_group("ramp_load");
    group.sample_size(50);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_time()
        .enable_io()
        .build()
        .unwrap();

    let (child, client, port) = rt.block_on(async move {
        let (child, port) = spawn_plugin("patui-template").await.unwrap();
        let client = connect_plugin(port).await;
        (child, client, port)
    });

    eprintln!("Setup patui-template: {:?}", child);

    group.bench_function("ramp_load_2", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let client = connect_plugin(port).await;
                test_plugin(client, 20000).await;
            });
        })
    });

    group.finish();

    rt.block_on(async move {
        shutdown_plugin(child, client).await;
    });
}

criterion_group!(ramp_load, bench_ramp_load);
criterion_group!(ramp_load_sleep, bench_ramp_load_sleep);

criterion_main!(ramp_load);
