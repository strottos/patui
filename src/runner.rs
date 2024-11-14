use crate::types::PatuiTest;

use eyre::Result;

struct TestRunner<'a> {
    test: &'a PatuiTest,
}

impl<'a> TestRunner<'a> {
    fn plan(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assertor::*

    use super::*

    #[test]
    fn plan_basic() {
        let test = PatuiTest {
            id: todo!(),
            details: todo!(),
        };
        let runner = TestRunner { test: &test };

        let ret = runner.plan();

        assert_that!(ret).is_equal_to(Ok(()));
    }
}
