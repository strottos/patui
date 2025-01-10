use std::{collections::HashMap, time::Duration};

use assertor::*;
use ptplugin::{
    plugin_server::{produce_results, run, shutdown, wait},
    run_plugin, PatuiData, PatuiDataInner, PatuiEvent, PatuiEventWithTimestamp, PatuiExpr,
};
use tokio::time::timeout;

#[tokio::test]
async fn transform_expr_string() {
    let (mut child, mut client) = run_plugin("patui-jq").await.unwrap();

    // Plugin will send us some results on this stream
    let res = client.produce_results(produce_results::Init {}).await;
    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let res = client
        .run(run::Request {
            function: "transform".to_string(),
            args: HashMap::from([(
                "in".to_string(),
                "\"{\\\"a\\\":1,\\\"b\\\":2}\"".to_string(),
            )]),
        })
        .await;

    assert_that!(res).is_ok();

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let response = response.unwrap();
    assert_that!(response.diagnostics).has_length(0);
    assert_that!(response.data).is_some();

    let event = PatuiEventWithTimestamp::try_from(response.data.unwrap());
    assert_that!(event).is_ok();
    let event = event.unwrap();
    assert_that!(event.value()).is_equal_to(&PatuiEvent::Results(
        PatuiExpr::try_from("out").unwrap(),
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
