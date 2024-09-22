// use std::sync::Arc;
//
// use crate::db::{Database, PatuiRun, PatuiStep};
//
// use eyre::Result;
//
// pub(crate) struct TestRunner<'a> {
//     pub(crate) run: &'a PatuiRun,
//     pub(crate) db: Arc<Database>,
// }
//
// impl TestRunner {
//     pub(crate) async fn run_test(&self) -> Result<()> {
//         let mut results = vec![];
//
//         for step in self.run.details.instance.steps.iter() {
//             let result = self.run_step(step).await?;
//             results.push(result);
//         }
//
//         Ok(())
//     }
//
//     async fn run_step(&self, step: &PatuiStep) -> Result<PatuiRunStepDetails> {
//         match step {
//             PatuiStep::Process(process) => self.run_process(process).await,
//             PatuiStep::Shell(_) => todo!(),
//             PatuiStep::Assertion(_) => todo!(),
//         }
//     }
//
//     async fn run_process(&self, process: &PatuiProcess) -> Result<PatuiRunStepDetails> {
//         let mut result = PatuiStepResult::default();
//
//         let mut cmd = Command::new(&step.command);
//         cmd.arg(&step.args);
//
//         let output = cmd.output()?;
//
//         result.output = output.stdout;
//         result.error = output.stderr;
//         result.exit_code = output.status.code().unwrap_or(-1);
//
//         Ok(result)
//     }
// }
