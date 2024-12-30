#[derive(Debug)]
pub(crate) struct PatuiStepRunnerWrite {}

impl PatuiStepRunnerWrite {
    pub(crate) fn new(
        step_name: String,
        patui_step_write: &crate::templates::PatuiStepWrite,
    ) -> Self {
        Self {}
    }
}
