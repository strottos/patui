use eyre::Result;

use crate::types::PatuiStepAssertion;

use super::PatuiStepRunnerTrait;

pub(crate) struct PatuiStepRunnerAssertion {
    step: PatuiStepAssertion,
}

impl PatuiStepRunnerAssertion {
    pub(crate) fn new(step: &PatuiStepAssertion) -> Self {
        Self { step: step.clone() }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerAssertion {}

#[cfg(test)]
mod tests {
    use super::*;

    use assertor::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn assert_that_is_ok() {
        let assertion = PatuiStepAssertion {
            expr: "foo".try_into().unwrap(),
        };

        let mut runner = PatuiStepRunnerAssertion::new(&assertion);

        assert_that!(runner.run()).is_ok();
    }
}
