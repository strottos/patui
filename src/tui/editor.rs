use color_eyre::Result;

use crate::types::PatuiTest;

pub(crate) fn create_test() -> Result<PatuiTest> {
    let template = crate::types::PatuiTest::default().to_editable_yaml_string()?;
    let test = PatuiTest::edit_yaml(template)?;

    Ok(test)
}

pub(crate) fn edit_test(test: PatuiTest) -> Result<PatuiTest> {
    let template = test.to_editable_yaml_string()?;
    let test = PatuiTest::edit_yaml(template)?;

    Ok(test)
}
