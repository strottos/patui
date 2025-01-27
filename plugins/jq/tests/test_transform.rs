use std::{collections::HashMap, time::Duration};

use assertor::*;
use tokio::time::timeout;
use tonic::Request;
use tracing_test::traced_test;

use ptplugin::{
    async_stream,
    plugin_server::{
        ack_result, produce_results, receive_results, run, shutdown, wait, ResultType,
    },
    run_plugin, PatuiData, PatuiDataInner, PatuiEvent, PatuiEventWithTimestamp, PatuiExpr,
};

// #[traced_test]
// #[tokio::test]
// async fn transform_expr_string() {
//     let (mut child, mut client) = run_plugin("patui-jq").await.unwrap();
//
//     // Plugin will send us some results on this stream
//     let res = timeout(
//         Duration::from_secs(1),
//         client.produce_results(produce_results::Init {}),
//     )
//     .await;
//     assert_that!(res).is_ok();
//     let res = res.unwrap();
//     assert_that!(res).is_ok();
//     let mut subscription_rx = res.unwrap().into_inner();
//
//     let res = client
//         .run(run::Request {
//             function: "transform".to_string(),
//             args: HashMap::from([(
//                 "in".to_string(),
//                 "\"{\\\"a\\\":1,\\\"b\\\":2}\"".to_string(),
//             )]),
//         })
//         .await;
//
//     assert_that!(res).is_ok();
//
//     let client_clone = client.clone();
//
//     tokio::spawn(async move {
//         let mut client = client_clone;
//         let outbound = async_stream::stream! {
//             // No results to send, but still need to trigger this
//             for results in [] {
//                 let results: PatuiData = results;
//                 tracing::trace!("Got results from receiver: {:?}", results);
//
//                 yield receive_results::Request {
//                     step_name: "transform".to_string(),
//                     function_name: "transform".to_string(),
//                     result_name: "out".to_string(),
//                     r#type: ResultType::Append.into(),
//                     results: Some(results.try_into().unwrap()),
//                 }
//             }
//         };
//
//         let response = client
//             .receive_results(Request::new(outbound))
//             .await
//             .unwrap()
//             .into_inner();
//
//         tracing::trace!("RESP = {:?}", response);
//     });
//
//     let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
//     assert_that!(response).is_ok();
//     let response = response.unwrap();
//     assert_that!(response).is_ok();
//     let response = response.unwrap();
//     assert_that!(response).is_some();
//     let response = response.unwrap();
//     assert_that!(response.diagnostics).has_length(0);
//     assert_that!(response.data).is_some();
//     let id = response.id;
//     let data = response.data;
//
//     let response = timeout(
//         Duration::from_secs(1),
//         client.ack_result(ack_result::Request { id }),
//     )
//     .await;
//     assert_that!(response).is_ok();
//     let response = response.unwrap();
//     assert_that!(response).is_ok();
//
//     let event = PatuiEventWithTimestamp::try_from(data.unwrap());
//     assert_that!(event).is_ok();
//     let event = event.unwrap();
//     assert_that!(event.value()).is_equal_to(&PatuiEvent::Results(
//         PatuiExpr::try_from("out").unwrap(),
//         true.into(),
//         ResultType::Append.into(),
//         PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//             (
//                 "a".to_string(),
//                 PatuiData::Known(PatuiDataInner::Integer(1)),
//             ),
//             (
//                 "b".to_string(),
//                 PatuiData::Known(PatuiDataInner::Integer(2)),
//             ),
//         ]))),
//     ));
//
//     let res = client.wait(wait::Request {}).await;
//
//     assert_that!(res).is_ok();
//     assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);
//
//     client.shutdown(shutdown::Request {}).await.unwrap();
//
//     // Wait for the server to shutdown
//     assert_that!(child.kill()).is_ok();
//     assert_that!(child.wait()).is_ok();
// }

#[traced_test]
#[tokio::test]
async fn transform_results_stream() {
    let (mut child, mut client) = run_plugin("patui-jq").await.unwrap();

    // Plugin will send us some results on this stream
    let res = timeout(
        Duration::from_secs(1),
        client.produce_results(produce_results::Init {}),
    )
    .await;
    assert_that!(res).is_ok();
    let res = res.unwrap();
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let res = client
        .run(run::Request {
            function: "transform".to_string(),
            args: HashMap::from([("in".to_string(), "steps.foo.bar.results".to_string())]),
        })
        .await;

    assert_that!(res).is_ok();

    let client_clone = client.clone();

    tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            for results in [
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    ("a".to_string(), PatuiData::Known(PatuiDataInner::Integer(1))),
                    ("b".to_string(), PatuiData::Known(PatuiDataInner::Integer(2))),
                ]))),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    ("a".to_string(), PatuiData::Known(PatuiDataInner::Integer(2))),
                    ("b".to_string(), PatuiData::Known(PatuiDataInner::Integer(3))),
                ]))),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    ("a".to_string(), PatuiData::Known(PatuiDataInner::Integer(3))),
                    ("b".to_string(), PatuiData::Known(PatuiDataInner::Integer(4))),
                ]))),
            ] {
                let results: PatuiData = results;
                tracing::trace!("Got results from receiver: {:?}", results);

                yield receive_results::Request {
                    step_name: "foo".to_string(),
                    function_name: "bar".to_string(),
                    result_name: "results".to_string(),
                    r#type: ResultType::Append.into(),
                    results: Some(results.try_into().unwrap()),
                }
            }
        };

        let response = client
            .receive_results(Request::new(outbound))
            .await
            .unwrap()
            .into_inner();

        tracing::trace!("RESP = {:?}", response);
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
    let id = response.id;
    let data = response.data;

    let response = timeout(
        Duration::from_secs(1),
        client.ack_result(ack_result::Request { id }),
    )
    .await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();

    let event = PatuiEventWithTimestamp::try_from(data.unwrap());
    assert_that!(event).is_ok();
    let event = event.unwrap();
    assert_that!(event.value()).is_equal_to(&PatuiEvent::Results(
        PatuiExpr::try_from("out").unwrap(),
        true.into(),
        ResultType::Append.into(),
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
    ));

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}
