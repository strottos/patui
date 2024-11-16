use self::process::PatuiStepRunnerProcess;

mod process;

pub(crate) enum PatuiStepRunnerFlavour {
    Process(PatuiStepRunnerProcess),
}

pub(crate) struct PatuiStepRunner {
    flavour: PatuiStepRunnerFlavour,
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    #[test]
    fn step_process() {}
}
