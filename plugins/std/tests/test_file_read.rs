use std::{collections::HashMap, time::Duration};

use assertor::*;
use tokio::time::timeout;

use patui_plugin_test::run_plugin;
use ptplugin::{
    plugin_server::{run, shutdown, wait},
    PatuiData, PatuiDataInner, PatuiEvent, PatuiEventWithTimestamp,
};

#[tokio::test]
async fn file_read() {
    let (mut child, mut client) = run_plugin("patui-std").await.unwrap();

    let res = client
        .run(run::Request {
            function: "file_read".to_string(),
            args: HashMap::from([("path".to_string(), "../../tests/data/test.txt".to_string())]),
        })
        .await;

    assert_that!(res).is_ok();
    let mut subscription_rx = res.unwrap().into_inner();

    let response = timeout(Duration::from_secs(2), subscription_rx.message()).await;
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_ok();
    let response = response.unwrap();
    assert_that!(response).is_some();
    let response = response.unwrap();
    assert_that!(response.diagnostics).has_length(0);
    assert_that!(response.name).is_equal_to("file_read".to_string());
    assert_that!(response.data).is_some();

    let event = PatuiEventWithTimestamp::try_from(response.data.unwrap());
    assert_that!(event).is_ok();
    let event = event.unwrap();
    assert_that!(event.value()).is_equal_to(&PatuiEvent::Results(
        "file_data".to_string(),
        PatuiData::Known(PatuiDataInner::String(
            std::fs::read_to_string("../../tests/data/test.txt").unwrap(),
        )),
    ));

    let res = client.wait(wait::Request {}).await;

    assert_that!(res).is_ok();
    assert_that!(res.unwrap().into_inner().diagnostics).has_length(0);

    client.shutdown(shutdown::Request {}).await.unwrap();

    // Wait for the server to shutdown
    assert_that!(child.kill()).is_ok();
    assert_that!(child.wait()).is_ok();
}
