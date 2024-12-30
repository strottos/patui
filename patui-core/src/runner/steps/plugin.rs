#[derive(Debug)]
pub(crate) struct PatuiStepRunnerPlugin {}

impl PatuiStepRunnerPlugin {
    pub(crate) fn new(
        step_name: String,
        patui_step_plugin: &crate::templates::PatuiStepPlugin,
    ) -> Self {
        Self {}
    }
}
