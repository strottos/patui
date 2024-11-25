use eyre::Result;

use crate::types::{PatuiStepAssertion, PatuiStepRead, PatuiStepWrite};

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

pub(crate) struct PatuiStepRunnerRead {
    step: PatuiStepRead,
}

impl PatuiStepRunnerRead {
    pub(crate) fn new(step: &PatuiStepRead) -> Self {
        Self { step: step.clone() }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerRead {}

pub(crate) struct PatuiStepRunnerWrite {
    step: PatuiStepWrite,
}

impl PatuiStepRunnerWrite {
    pub(crate) fn new(step: &PatuiStepWrite) -> Self {
        Self { step: step.clone() }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerWrite {}

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
