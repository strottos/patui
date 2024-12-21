use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use eyre::{eyre, Result};
use tokio::{
    sync::{broadcast, mpsc, RwLock},
    task::JoinHandle,
};

use super::{init_subscribe_steps, PatuiStepRunner, PatuiStepRunnerTrait};
use crate::types::{
    expr::ast::{Expr, ExprKind, Lit, LitKind, Term, TermParts},
    PatuiEvent, PatuiEventKind, PatuiStepAssertion, PatuiStepData, PatuiStepDataFlavour,
};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerAssertion {
    step_name: String,
    step: PatuiStepAssertion,

    receivers: Option<HashMap<Expr, broadcast::Receiver<PatuiStepData>>>,

    tasks: Vec<JoinHandle<()>>,

    results: Arc<RwLock<HashMap<Expr, Vec<PatuiStepData>>>>,
}

impl PatuiStepRunnerAssertion {
    pub(crate) fn new(step_name: String, step: &PatuiStepAssertion) -> Self {
        Self {
            step_name,
            step: step.clone(),
            receivers: None,
            tasks: vec![],
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerAssertion {
    async fn init(
        &mut self,
        current_step_name: &str,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        let receivers =
            init_subscribe_steps(&self.step.expr, current_step_name, &step_runners).await?;
        self.receivers = Some(receivers);

        Ok(())
    }

    fn run(&mut self, tx: mpsc::Sender<PatuiEvent>) -> Result<()> {
        let step_name = self.step_name.clone();
        let step = self.step.clone();
        let receivers = self.receivers.take();
        let results = self.results.clone();

        // Notify of results coming in
        let (notify_tx, mut notify_rx) = mpsc::channel(1);

        let task = tokio::spawn(async move {
            // Analyze results if possible, no guarantees it will be and if we haven't received
            // enough data we go back to waiting.
            let results = results.clone();
            let expr = &step.expr.expr;
            while let Some(_) = notify_rx.recv().await {
                let results = results.read().await.clone();
                match eval(&expr, &results) {
                    Ok(EvalResult::Known(patui_step_data)) => match patui_step_data.data {
                        PatuiStepDataFlavour::Bool(b) => {
                            if b {
                                tx.send(PatuiEvent::new(
                                    PatuiEventKind::Log(format!("Assertion passed: {:?}", expr,)),
                                    step_name.clone(),
                                ))
                                .await
                                .unwrap();
                            } else {
                                tx.send(PatuiEvent::new(
                                    PatuiEventKind::Failure(format!(
                                        "Assertion failed: {:?}",
                                        expr,
                                    )),
                                    step_name.clone(),
                                ))
                                .await
                                .unwrap();
                            }
                        }
                        _ => tx
                            .send(PatuiEvent::new(
                                PatuiEventKind::Error(format!(
                                    "Assertion evaluated to non-boolean: {:?}",
                                    patui_step_data.data
                                )),
                                step_name.clone(),
                            ))
                            .await
                            .unwrap(),
                    },
                    Ok(EvalResult::Predictable(_patui_step_data)) => todo!(),
                    Ok(EvalResult::Unknown) => {}
                    Err(_err) => tx
                        .send(PatuiEvent::new(
                            PatuiEventKind::Failure("Assertion failure".to_string()),
                            step_name.clone(),
                        ))
                        .await
                        .unwrap(),
                }
            }
        });

        self.tasks.push(task);

        let results = self.results.clone();

        let task = tokio::spawn(async move {
            let Some(receivers) = receivers else {
                panic!("No receivers found");
            };
            let results = results;
            let notify_tx = notify_tx.clone();

            let mut tasks = vec![];

            for (expr, mut receiver) in receivers.into_iter() {
                tracing::trace!("Checking receiver for expr: {:?}", expr);

                let results = results.clone();
                let notify_tx = notify_tx.clone();

                tasks.push(tokio::spawn(async move {
                    let results = results.clone();
                    while let Ok(data) = receiver.recv().await {
                        tracing::trace!("Received data: {:?}", data);
                        let mut lock = results.write().await;
                        let entry = lock.entry(expr.clone()).or_insert(vec![]);
                        entry.push(data);
                        drop(lock);
                        notify_tx.clone().send(()).await.unwrap();
                    }
                }));
            }

            for task in tasks.drain(..) {
                task.await.unwrap();
            }
        });

        self.tasks.push(task);

        Ok(())
    }

    async fn wait(&mut self) -> Result<()> {
        tracing::trace!("Waiting");

        for task in self.tasks.drain(..) {
            task.await?;
        }

        Ok(())
    }

    #[cfg(test)]
    fn test_set_receiver(
        &mut self,
        sub_ref: &str,
        rx: broadcast::Receiver<PatuiStepData>,
    ) -> Result<()> {
        use super::PatuiExpr;

        let sub_ref_expr: PatuiExpr = sub_ref.try_into()?;
        let receivers = HashMap::from([(sub_ref_expr.expr, rx)]);
        self.receivers = Some(receivers);

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
enum EvalResult {
    /// We can clearly say we have a successful result of the evaluation and it is independent of
    /// any future results that may come in. E.g. `steps.foo.out[0] == "hello"` is true, any
    /// further results coming in won't effect this as they'll be a different index.
    Known(PatuiStepData),
    /// The evaluation can be determined at this time but that result may change as further results
    /// come in. If it stays in this state after all results are in the result is confirmed. E.g.
    /// consider `steps.foo.out.len() == 4`, if we only have 3 results currently we could get
    /// another result come in or it could be we're done in which case this is false.
    Predictable(PatuiStepData),
    /// We have no idea what the result is currently and if all results are in then this is an
    /// error. E.g. consider `steps.foo.out[4] == "foo"` but we only have 3 results in so far, then
    /// we could get another result in and then we can tell what the result is, or we are done and
    /// then we have an error.
    Unknown,
}

impl EvalResult {
    fn get_step_data(&self) -> Result<&PatuiStepData> {
        match self {
            EvalResult::Known(patui_step_data) => Ok(patui_step_data),
            EvalResult::Predictable(patui_step_data) => Ok(patui_step_data),
            EvalResult::Unknown => Err(eyre!("Result currently not available")),
        }
    }
}

fn eval(expr: &Expr, results: &HashMap<Expr, Vec<PatuiStepData>>) -> Result<EvalResult> {
    tracing::trace!("Evaluating expr: {:#?}", expr);

    match expr {
        Expr {
            kind: ExprKind::BinOp(bin_op, lhs, rhs),
        } => match bin_op {
            crate::types::expr::ast::BinOp::Add => todo!(),
            crate::types::expr::ast::BinOp::Subtract => todo!(),
            crate::types::expr::ast::BinOp::Multiply => todo!(),
            crate::types::expr::ast::BinOp::Divide => todo!(),
            crate::types::expr::ast::BinOp::Modulo => todo!(),
            crate::types::expr::ast::BinOp::And => todo!(),
            crate::types::expr::ast::BinOp::Or => todo!(),
            crate::types::expr::ast::BinOp::Equal => {
                let lhs_eval = eval(&*lhs, results)?;
                let rhs_eval = eval(&*rhs, results)?;
                match lhs_eval {
                    EvalResult::Known(lhs_data) => match rhs_eval {
                        EvalResult::Known(rhs_data) => Ok(EvalResult::Known(PatuiStepData::new(
                            PatuiStepDataFlavour::Bool(lhs_data == rhs_data),
                        ))),
                        EvalResult::Predictable(rhs_data) => Ok(EvalResult::Predictable(
                            PatuiStepData::new(PatuiStepDataFlavour::Bool(lhs_data == rhs_data)),
                        )),
                        EvalResult::Unknown => Err(eyre!("Unknown result")),
                    },
                    EvalResult::Predictable(lhs_data) => match rhs_eval {
                        EvalResult::Known(rhs_data) => Ok(EvalResult::Predictable(
                            PatuiStepData::new(PatuiStepDataFlavour::Bool(lhs_data == rhs_data)),
                        )),
                        EvalResult::Predictable(rhs_data) => Ok(EvalResult::Predictable(
                            PatuiStepData::new(PatuiStepDataFlavour::Bool(lhs_data == rhs_data)),
                        )),
                        EvalResult::Unknown => Err(eyre!("Unknown result")),
                    },
                    EvalResult::Unknown => Err(eyre!("Unknown result")),
                }
            }
            crate::types::expr::ast::BinOp::NotEqual => todo!(),
            crate::types::expr::ast::BinOp::LessThan => todo!(),
            crate::types::expr::ast::BinOp::LessThanEqual => todo!(),
            crate::types::expr::ast::BinOp::GreaterThan => todo!(),
            crate::types::expr::ast::BinOp::GreaterThanEqual => todo!(),
            crate::types::expr::ast::BinOp::Contains => todo!(),
            crate::types::expr::ast::BinOp::NotContains => todo!(),
        },
        Expr {
            kind: ExprKind::UnOp(_un_op, _p),
        } => todo!(),
        Expr {
            kind: ExprKind::Lit(lit),
        } => Ok(EvalResult::Known(eval_lit(lit, results)?)),
        Expr {
            kind: ExprKind::Term(Term { values, .. }),
        } => match values.first().as_ref() {
            Some(&TermParts::Ident(ident)) => match &ident[..] {
                "steps" => {
                    let Some(relevant_parts) = values.get(0..3) else {
                        return Err(eyre!("Not enough parts in term to evaluate: {:?}", values));
                    };
                    if let Some(result) = results.get(&Expr {
                        kind: ExprKind::Term(Term {
                            values: relevant_parts.to_vec(),
                        }),
                    }) {
                        let index = match values.get(3) {
                            Some(TermParts::Index(i)) => *i,
                            _ => Err(eyre!("No index in term: {:?}", values))?,
                        };

                        match result.get(index) {
                            Some(data) => Ok(EvalResult::Known(data.clone())),
                            None => Ok(EvalResult::Unknown),
                        }
                    } else {
                        Err(eyre!("No data for term"))
                    }
                }
                _ => Err(eyre!("Unknown term first element: {:?}", values.first())),
            },
            _ => Err(eyre!("Unknown term first element: {:?}", values.first())),
        },
        Expr {
            kind: ExprKind::If(_p1, _p2, _p3),
        } => todo!(),
    }
}

fn eval_lit(lit: &Lit, results: &HashMap<Expr, Vec<PatuiStepData>>) -> Result<PatuiStepData> {
    match &lit.kind {
        LitKind::Null => Ok(PatuiStepData::new(PatuiStepDataFlavour::Null)),
        LitKind::Bool(b) => Ok(PatuiStepData::new(PatuiStepDataFlavour::Bool(b.clone()))),
        LitKind::Bytes(bytes) => Ok(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
            bytes.clone(),
        ))),
        LitKind::Integer(i) => Ok(PatuiStepData::new(PatuiStepDataFlavour::Integer(i.clone()))),
        LitKind::Decimal(f) => Ok(PatuiStepData::new(PatuiStepDataFlavour::Float(f.clone()))),
        LitKind::Str(s) => Ok(PatuiStepData::new(PatuiStepDataFlavour::String(s.clone()))),
        LitKind::List(vec) => Ok(PatuiStepData::new(PatuiStepDataFlavour::Array(
            vec.iter()
                .map(|lit| eval(lit, results))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|result| result.get_step_data().unwrap().data.clone())
                .collect(),
        ))),
        LitKind::Map(map) => {
            let map = map
                .iter()
                .map(|map| {
                    let (key, value) = &**map;
                    match (key, eval(value, results)) {
                        (
                            Expr {
                                kind:
                                    ExprKind::Lit(Lit {
                                        kind: LitKind::Str(s),
                                    }),
                            },
                            Ok(value),
                        ) => Ok((s.clone(), value.get_step_data().unwrap().data.clone())),
                        _ => Err(eyre!("Invalid key/value in map: {:?} - {:?}", key, value)),
                    }
                })
                .collect::<Result<HashMap<_, _>>>()?;

            Ok(PatuiStepData::new(PatuiStepDataFlavour::Map(map)))
        }
        LitKind::Set(vec) => Ok(PatuiStepData::new(PatuiStepDataFlavour::Set(
            vec.iter()
                .map(|lit| eval(lit, results))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|result| result.get_step_data().unwrap().data.clone())
                .collect(),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use assertor::*;
    use bytes::Bytes;
    use tokio::{sync::mpsc, time::timeout};
    use tracing_test::traced_test;

    use crate::types::{PatuiEventKind, PatuiExpr};

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn single_channel_read_and_eval_null() {
        let mut main_step = PatuiStepRunnerAssertion::new(
            "main".to_string(),
            &PatuiStepAssertion {
                expr: "steps.test_input.out[0] == null".try_into().unwrap(),
            },
        );

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Null))
            .unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let res = timeout(Duration::from_millis(500), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(matches!(res.value(), PatuiEventKind::Log(_))).is_equal_to(true);
    }

    #[traced_test]
    #[tokio::test]
    async fn single_channel_read_and_eval_null_fails() {
        let mut main_step = PatuiStepRunnerAssertion::new(
            "main".to_string(),
            &PatuiStepAssertion {
                expr: "steps.test_input.out[0] == null".try_into().unwrap(),
            },
        );

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                Bytes::from("ABC"),
            )))
            .unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_err();

        let res = timeout(Duration::from_millis(500), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(matches!(res.value(), PatuiEventKind::Failure(_))).is_equal_to(true);
    }

    #[traced_test]
    #[test]
    fn evaluate_result_without_sub() {
        let expr: PatuiExpr = "steps.test_input.out[0]".try_into().unwrap();

        let ret = super::eval(&expr.expr, &HashMap::from([]));

        assert_that!(ret).is_err();
    }

    #[traced_test]
    #[test]
    fn evaluate_result_without_result() {
        let expr: PatuiExpr = "steps.test_input.out[0]".try_into().unwrap();

        let key: PatuiExpr = "steps.test_input.out".try_into().unwrap();

        let ret = super::eval(&expr.expr, &HashMap::from([(key.expr, vec![])]));

        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_that!(ret).is_equal_to(EvalResult::Unknown);
    }

    #[traced_test]
    #[test]
    fn evaluate_result_with_null() {
        let expr: PatuiExpr = "steps.test_input.out[0]".try_into().unwrap();

        let key: PatuiExpr = "steps.test_input.out".try_into().unwrap();

        let ret = super::eval(
            &expr.expr,
            &HashMap::from([(
                key.expr,
                vec![PatuiStepData::new(PatuiStepDataFlavour::Null)],
            )]),
        );

        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        let ret = ret.get_step_data();
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap().data).is_equal_to(&PatuiStepDataFlavour::Null);
    }

    #[traced_test]
    #[test]
    fn evaluate_lits() {
        for (expr_str, expected) in [
            ("true", PatuiStepDataFlavour::Bool(true)),
            ("false", PatuiStepDataFlavour::Bool(false)),
            ("null", PatuiStepDataFlavour::Null),
            (
                "b[1,2,3]",
                PatuiStepDataFlavour::Bytes(Bytes::from(vec![1, 2, 3])),
            ),
            (
                "\"hello\"",
                PatuiStepDataFlavour::String("hello".to_string()),
            ),
            ("123", PatuiStepDataFlavour::Integer("123".to_string())),
            (
                "123.456",
                PatuiStepDataFlavour::Float("123.456".to_string()),
            ),
            (
                "[1,2,3]",
                PatuiStepDataFlavour::Array(vec![
                    PatuiStepDataFlavour::Integer("1".to_string()),
                    PatuiStepDataFlavour::Integer("2".to_string()),
                    PatuiStepDataFlavour::Integer("3".to_string()),
                ]),
            ),
            (
                "{\"a\": 1, \"b\": 2}",
                PatuiStepDataFlavour::Map(HashMap::from([
                    (
                        "a".to_string(),
                        PatuiStepDataFlavour::Integer("1".to_string()),
                    ),
                    (
                        "b".to_string(),
                        PatuiStepDataFlavour::Integer("2".to_string()),
                    ),
                ])),
            ),
            (
                "{1,2,3}",
                PatuiStepDataFlavour::Set(vec![
                    PatuiStepDataFlavour::Integer("1".to_string()),
                    PatuiStepDataFlavour::Integer("2".to_string()),
                    PatuiStepDataFlavour::Integer("3".to_string()),
                ]),
            ),
        ] {
            let expr: PatuiExpr = expr_str.try_into().unwrap();

            let ret = super::eval(&expr.expr, &HashMap::from([]));

            assert_that!(ret).is_ok();
            let ret = ret.unwrap();
            let ret = ret.get_step_data();
            assert_that!(ret).is_ok();
            assert_that!(ret.unwrap().data).is_equal_to(&expected);
        }
    }

    #[traced_test]
    #[test]
    fn evaluate_null_result_equals_null() {
        let expr: PatuiExpr = "steps.test_input.out[0] == null".try_into().unwrap();

        let key: PatuiExpr = "steps.test_input.out".try_into().unwrap();

        let ret = super::eval(
            &expr.expr,
            &HashMap::from([(
                key.expr,
                vec![PatuiStepData::new(PatuiStepDataFlavour::Null)],
            )]),
        );

        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        let ret = ret.get_step_data();
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap().data).is_equal_to(&PatuiStepDataFlavour::Bool(true));
    }
}
