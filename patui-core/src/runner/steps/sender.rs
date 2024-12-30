#[derive(Debug)]
pub(crate) struct PatuiStepRunnerSender {}

impl PatuiStepRunnerSender {
    pub(crate) fn new(
        step_name: String,
        patui_step_sender: &crate::templates::PatuiStepSender,
    ) -> Self {
        Self {}
    }
}
