use std::{collections::HashMap, sync::Arc};

use thiserror::Error;
use tokio::sync::RwLock;

use crate::{
    expr::{PatuiData, PatuiExprError},
    templates::PatuiStep,
};

#[derive(Debug)]
pub(crate) struct PatuiStepRunner {
    step: PatuiStep,
    results: Arc<RwLock<HashMap<String, Vec<PatuiData>>>>,
}

impl PatuiStepRunner {
    pub(crate) fn new(
        step: &PatuiStep,
        results: Arc<RwLock<HashMap<String, Vec<PatuiData>>>>,
    ) -> Self {
        Self {
            step: step.clone(),
            results,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum PatuiStepRunnerError {
    #[error("Subscription not supported: {0}")]
    SubscriptionNotSupported(String),
    #[error("Test set receiver not supported")]
    TestSetReceiverNotSupported,
    #[error("Expression error: {0}")]
    ExprError(#[from] PatuiExprError),
}
