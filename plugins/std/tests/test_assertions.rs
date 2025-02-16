use std::{collections::HashMap, time::Duration};

use assertor::*;
use tokio::time::timeout;
use tonic::Request;
use tracing_test::traced_test;

use ptplugin::{
    async_stream, check_event_response,
    plugin_server::{receive_results, run, shutdown, ResultType},
    run_plugin, shutdown_plugin, PatuiData, PatuiDataInner, PatuiEvent, PatuiEventWithTimestamp,
    PatuiExpr, PatuiResultType, PatuiResultTypeConfirm,
};

#[traced_test]
#[tokio::test]
async fn assertion_simple() {
    let (mut child, mut client) = run_plugin("patui-std").await.unwrap();

    let client_clone = client.clone();

    // Send results to the plugin
    let results_to_plugin_task = tokio::spawn(async move {
        let mut client = client_clone;
        let outbound = async_stream::stream! {
            // No results to send, but still need to trigger this
            for results in [] {
                let results: PatuiData = results;
                tracing::trace!("Got results from receiver: {:?}", results);

                yield receive_results::Request {
                    step_name: "assertion".to_string(),
                    function_name: "assertion".to_string(),
                    result_name: "eval".to_string(),
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
    let ret = results_to_plugin_task.await;
    assert_that!(ret).is_ok();
    let ret = ret.unwrap();
    drop(ret);

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Result(
        PatuiExpr::try_from("assertion").unwrap(),
        true.into(),
        PatuiResultType::List(0),
        PatuiData::Known(PatuiDataInner::Bool(true)),
    ));

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    tracing::info!("Got response from plugin: {:?}", response);
    let event = check_event_response(response).await;
    assert_that!(event).is_equal_to(PatuiEvent::Done(PatuiResultTypeConfirm::List(1)));

    shutdown_plugin(child, client).await;
}

// #[traced_test]
// #[tokio::test]
// async fn assertion_simple_failure() {
//     let (mut child, mut client) = run_plugin("patui-std").await.unwrap();
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
//             function: "assertion".to_string(),
//             args: HashMap::from([("expr".to_string(), "[1,2,3][1] == 3".to_string())]),
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
//                     step_name: "assertion".to_string(),
//                     function_name: "assertion".to_string(),
//                     result_name: "eval".to_string(),
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
//         PatuiExpr::try_from("assertion").unwrap(),
//         false.into(),
//         ResultType::Append.into(),
//         PatuiData::Known(PatuiDataInner::Bool(false)),
//     ));
//
//     let res = timeout(Duration::from_secs(2), client.wait(wait::Request {})).await;
//
//     assert_that!(res).is_ok();
//     let res = res.unwrap();
//     assert_that!(res).is_ok();
//     assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);
//
//     client.shutdown(shutdown::Request {}).await.unwrap();
//
//     // Wait for the server to shutdown
//     assert_that!(child.kill()).is_ok();
//     assert_that!(child.wait()).is_ok();
// }

// #[traced_test]
// #[tokio::test]
// async fn assertion_simple_error() {
//     let (mut child, mut client) = run_plugin("patui-std").await.unwrap();
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
//             function: "assertion".to_string(),
//             args: HashMap::from([("expr".to_string(), "[1,2,3][1]".to_string())]),
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
//                     step_name: "assertion".to_string(),
//                     function_name: "assertion".to_string(),
//                     result_name: "eval".to_string(),
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
//     assert_that!(event.value()).is_equal_to(&PatuiEvent::Error(
//         "Assertion error, evaluated to type Integer: [1,2,3][1]".to_string(),
//     ));
//
//     let res = timeout(Duration::from_secs(2), client.wait(wait::Request {})).await;
//
//     assert_that!(res).is_ok();
//     let res = res.unwrap();
//     assert_that!(res).is_ok();
//     assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);
//
//     client.shutdown(shutdown::Request {}).await.unwrap();
//
//     // Wait for the server to shutdown
//     assert_that!(child.kill()).is_ok();
//     assert_that!(child.wait()).is_ok();
// }
//
// #[traced_test]
// #[tokio::test]
// async fn assertion_with_idents_and_no_results() {
//     let (mut child, mut client) = run_plugin("patui-std").await.unwrap();
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
//             function: "assertion".to_string(),
//             args: HashMap::from([(
//                 "expr".to_string(),
//                 "(my.foo.results.len() == 1) && (my.foo.results[0] == 1)".to_string(),
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
//             for results in [] {
//                 let results: PatuiData = results;
//                 tracing::trace!("Got results from receiver: {:?}", results);
//
//                 yield receive_results::Request {
//                     step_name: "assertion".to_string(),
//                     function_name: "assertion".to_string(),
//                     result_name: "eval".to_string(),
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
//     assert_that!(response).is_none();
//
//     let res = timeout(Duration::from_secs(2), client.wait(wait::Request {})).await;
//
//     assert_that!(res).is_ok();
//     let res = res.unwrap();
//     assert_that!(res).is_ok();
//     assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);
//
//     client.shutdown(shutdown::Request {}).await.unwrap();
//
//     // Wait for the server to shutdown
//     assert_that!(child.kill()).is_ok();
//     assert_that!(child.wait()).is_ok();
// }

// #[traced_test]
// #[tokio::test]
// async fn assertion_with_idents_and_not_enough_results() {
//     let (mut child, mut client) = run_plugin("patui-std").await.unwrap();
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
//             function: "assertion".to_string(),
//             args: HashMap::from([(
//                 "expr".to_string(),
//                 "(my.foo.results.len() == 3) && (my.foo.results[2] == 1)".to_string(),
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
//             for results in [PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                 ("my".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                     ("foo".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                         ("results".to_string(), PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(PatuiDataInner::Integer(1))]))),
//                     ])))),
//                 ]))))
//             ])))] {
//                 let results: PatuiData = results;
//                 tracing::trace!("Got results from receiver: {:?}", results);
//
//                 yield receive_results::Request {
//                     step_name: "assertion".to_string(),
//                     function_name: "assertion".to_string(),
//                     result_name: "eval".to_string(),
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
//     assert_that!(response.data).is_some();
//     let results = response.data.unwrap().try_into().unwrap();
//     assert_that!(results).is_equal_to(PatuiEvent::Error("Data not found".to_string()));
//
//     let res = timeout(Duration::from_secs(2), client.wait(wait::Request {})).await;
//
//     assert_that!(res).is_ok();
//     let res = res.unwrap();
//     assert_that!(res).is_ok();
//     assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);
//
//     client.shutdown(shutdown::Request {}).await.unwrap();
//
//     // Wait for the server to shutdown
//     assert_that!(child.kill()).is_ok();
//     assert_that!(child.wait()).is_ok();
// }

// #[traced_test]
// #[tokio::test]
// async fn assertion_with_idents_and_correct_results() {
//     let (mut child, mut client) = run_plugin("patui-std").await.unwrap();
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
//             function: "assertion".to_string(),
//             args: HashMap::from([(
//                 "expr".to_string(),
//                 "(my.foo.results.len() == 1) && (my.foo.results[0] == 1)".to_string(),
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
//             for results in [PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                 ("my".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                     ("foo".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                         ("results".to_string(), PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(PatuiDataInner::Integer(1))]))),
//                     ])))),
//                 ]))))
//             ])))] {
//                 let results: PatuiData = results;
//                 tracing::trace!("Got results from receiver: {:?}", results);
//
//                 yield receive_results::Request {
//                     step_name: "assertion".to_string(),
//                     function_name: "assertion".to_string(),
//                     result_name: "eval".to_string(),
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
//     assert_that!(response.name).is_equal_to("eval".to_string());
//     assert_that!(response.data).is_some();
//     let id = response.id;
//     let name = response.name;
//     let data = response.data;
//
//     let response = timeout(
//         Duration::from_secs(1),
//         client.ack_result(ack_result::Request { id, name }),
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
//         PatuiExpr::try_from("assertion").unwrap(),
//         true.into(),
//         ResultType::Append.into(),
//         PatuiData::Known(PatuiDataInner::Bool(true)),
//     ));
//
//     let res = timeout(Duration::from_secs(2), client.wait(wait::Request {})).await;
//
//     assert_that!(res).is_ok();
//     let res = res.unwrap();
//     assert_that!(res).is_ok();
//     assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);
//
//     client.shutdown(shutdown::Request {}).await.unwrap();
//
//     // Wait for the server to shutdown
//     assert_that!(child.kill()).is_ok();
//     assert_that!(child.wait()).is_ok();
// }

// #[traced_test]
// #[tokio::test]
// async fn assertion_with_idents_and_incorrect_results() {
//     let (mut child, mut client) = run_plugin("patui-std").await.unwrap();
//
//     // Plugin will send us some results on this stream
//     let res = client.produce_results(produce_results::Init {}).await;
//     assert_that!(res).is_ok();
//     let mut subscription_rx = res.unwrap().into_inner();
//
//     let res = client
//         .run(run::Request {
//             function: "assertion".to_string(),
//             args: HashMap::from([(
//                 "expr".to_string(),
//                 "(my.foo.results.len() == 1) && (my.foo.results[0] == 1)".to_string(),
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
//             for results in [PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                 ("my".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                     ("foo".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                         ("results".to_string(), PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(PatuiDataInner::Integer(1))]))),
//                     ])))),
//                 ])))),
//                 ("my".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                     ("foo".to_string(), PatuiData::Known(PatuiDataInner::Map(HashMap::from([
//                         ("results".to_string(), PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(PatuiDataInner::Integer(1)), PatuiData::Known(PatuiDataInner::Integer(2))]))),
//                     ])))),
//                 ]))))
//             ])))] {
//                 let results: PatuiData = results;
//                 tracing::trace!("Got results from receiver: {:?}", results);
//
//                 yield receive_results::Request {
//                     step_name: "assertion".to_string(),
//                     function_name: "assertion".to_string(),
//                     result_name: "eval".to_string(),
//                     r#type: ResultType::Append.into(),
//                     results: Some(results.try_into().unwrap()),
//                 }
//             }
//         };
//
//         let mut response = client
//             .receive_results(Request::new(outbound))
//             .await
//             .unwrap()
//             .into_inner();
//
//         let Ok(Some(resp)) = response.message().await else {
//             panic!("No response");
//         };
//         tracing::trace!("RESP = {:?}", resp);
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
//
//     let event = PatuiEventWithTimestamp::try_from(response.data.unwrap());
//     assert_that!(event).is_ok();
//     let event = event.unwrap();
//     assert_that!(event.value()).is_equal_to(&PatuiEvent::Failure(
//         PatuiExpr::try_from("assertion").unwrap(),
//         "evaluated expr to false: (my.foo.results.len() == 1) && (my.foo.results[0] == 1)"
//             .to_string(),
//     ));
//
//     let res = timeout(Duration::from_secs(2), client.wait(wait::Request {})).await;
//
//     assert_that!(res).is_ok();
//     let res = res.unwrap();
//     assert_that!(res).is_ok();
//     assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);
//
//     client.shutdown(shutdown::Request {}).await.unwrap();
//
//     // Wait for the server to shutdown
//     assert_that!(child.kill()).is_ok();
//     assert_that!(child.wait()).is_ok();
// }
