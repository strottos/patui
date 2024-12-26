use std::collections::HashMap;

use eyre::{eyre, Result};

use crate::types::{expr::ast::*, PatuiStepData};

#[derive(Debug, PartialEq)]
pub(crate) enum EvalResult {
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

pub(crate) fn eval(expr: &Expr, results: &HashMap<Expr, Vec<PatuiStepData>>) -> Result<EvalResult> {
    tracing::trace!("Evaluating expr: {:#?}", expr);

    match expr {
        Expr {
            kind: ExprKind::BinOp(bin_op, lhs, rhs),
        } => eval_bin_op(bin_op, lhs, rhs, results),
        Expr {
            kind: ExprKind::UnOp(_un_op, _p),
        } => todo!(),
        Expr {
            kind: ExprKind::Term(Term { values, .. }),
        } => eval_term(values, results),
        Expr {
            kind: ExprKind::If(_p1, _p2, _p3),
        } => todo!(),
    }
}

fn eval_bin_op(
    bin_op: &BinOp,
    lhs: &P<Expr>,
    rhs: &P<Expr>,
    results: &HashMap<Expr, Vec<PatuiStepData>>,
) -> Result<EvalResult> {
    match bin_op {
        BinOp::Add => todo!(),
        BinOp::Subtract => todo!(),
        BinOp::Multiply => todo!(),
        BinOp::Divide => todo!(),
        BinOp::Modulo => todo!(),
        BinOp::And => todo!(),
        BinOp::Or => todo!(),
        BinOp::Equal => {
            let lhs_eval = eval(lhs, results)?;
            tracing::trace!("lhs_eval: {:?}", lhs_eval);
            let rhs_eval = eval(rhs, results)?;
            tracing::trace!("rhs_eval: {:?}", rhs_eval);
            match (lhs_eval, rhs_eval) {
                (EvalResult::Known(lhs_data), EvalResult::Known(rhs_data)) => {
                    Ok(EvalResult::Known(PatuiStepData::Bool(lhs_data == rhs_data)))
                }
                (EvalResult::Predictable(lhs_data), EvalResult::Known(rhs_data)) => Ok(
                    EvalResult::Predictable(PatuiStepData::Bool(lhs_data == rhs_data)),
                ),
                (EvalResult::Known(lhs_data), EvalResult::Predictable(rhs_data)) => Ok(
                    EvalResult::Predictable(PatuiStepData::Bool(lhs_data == rhs_data)),
                ),
                (EvalResult::Predictable(lhs_data), EvalResult::Predictable(rhs_data)) => Ok(
                    EvalResult::Predictable(PatuiStepData::Bool(lhs_data == rhs_data)),
                ),
                (EvalResult::Unknown, _) => Ok(EvalResult::Unknown),
                (_, EvalResult::Unknown) => Ok(EvalResult::Unknown),
            }
        }
        BinOp::NotEqual => todo!(),
        BinOp::LessThan => todo!(),
        BinOp::LessThanEqual => todo!(),
        BinOp::GreaterThan => todo!(),
        BinOp::GreaterThanEqual => todo!(),
        BinOp::Contains => todo!(),
        BinOp::NotContains => todo!(),
    }
}

fn eval_term(
    term_parts: &Vec<TermParts>,
    results: &HashMap<Expr, Vec<PatuiStepData>>,
) -> Result<EvalResult> {
    match term_parts.first().as_ref() {
        Some(&TermParts::Ident(ident)) => match &ident[..] {
            "steps" => {
                let Some(step_parts) = term_parts.get(0..3) else {
                    return Err(eyre!(
                        "Not enough parts in term to evaluate: {:?}",
                        term_parts
                    ));
                };
                if let Some(result) = results.get(&Expr {
                    kind: ExprKind::Term(Term {
                        values: step_parts.to_vec(),
                    }),
                }) {
                    tracing::trace!("Found result for step term {:?}: {:?}", step_parts, result);
                    let index = match term_parts.get(3) {
                        Some(i) => match i {
                            TermParts::Index(p) => match &**p {
                                Expr {
                                    kind: ExprKind::Term(Term { values }),
                                } => match &values[..] {
                                    [TermParts::Lit(Lit {
                                        kind: LitKind::Integer(i),
                                    })] => i.parse::<usize>().unwrap(),
                                    _ => todo!(),
                                },
                                _ => todo!(),
                            },
                            _ => todo!(),
                        },
                        _ => todo!(),
                    };

                    tracing::trace!("Index: {:?}", index);
                    tracing::trace!("Result: {:?}", result);

                    match result.get(index) {
                        Some(step_data) => {
                            tracing::trace!(
                                "Found result for step index term {:?}[{:?}]: {:?}",
                                step_parts,
                                index,
                                step_data
                            );
                            let ret = eval_step_data(step_data.clone(), term_parts.get(4..))?;
                            Ok(EvalResult::Known(ret))
                        }
                        None => Ok(EvalResult::Unknown),
                    }
                } else {
                    Err(eyre!("No data for term"))
                }
            }
            // Will there ever be anything beyond steps? Feels unlikely... if so can remove
            // this keyword later.
            _ => Err(eyre!("Unknown term first element: {:?}", ident)),
        },
        Some(&TermParts::Lit(lit)) => {
            let step_data = eval_lit(lit, results)?;
            let ret = eval_step_data(step_data, term_parts.get(1..))?;
            Ok(EvalResult::Known(ret))
        }
        _ => Err(eyre!("Term first element not found: {:?}", term_parts)),
    }
}

fn eval_lit(lit: &Lit, results: &HashMap<Expr, Vec<PatuiStepData>>) -> Result<PatuiStepData> {
    match &lit.kind {
        LitKind::Null => Ok(PatuiStepData::Null),
        LitKind::Bool(b) => Ok(PatuiStepData::Bool(*b)),
        LitKind::Bytes(bytes) => Ok(PatuiStepData::Bytes(bytes.clone())),
        LitKind::Integer(i) => Ok(PatuiStepData::Integer(i.clone())),
        LitKind::Decimal(f) => Ok(PatuiStepData::Float(f.clone())),
        LitKind::Str(s) => Ok(PatuiStepData::String(s.clone())),
        LitKind::List(vec) => Ok(PatuiStepData::Array(
            vec.iter()
                .map(|lit| eval(lit, results))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|result| result.get_step_data().unwrap().clone())
                .collect(),
        )),
        LitKind::Map(map) => {
            let map = map
                .iter()
                .map(|map| {
                    let (key, value) = &**map;
                    match (key, eval(value, results)) {
                        (
                            Expr {
                                kind: ExprKind::Term(Term { values }),
                            },
                            Ok(value),
                        ) => {
                            if values.len() != 1 {
                                return Err(eyre!("Invalid key in map: {:?}", values));
                            }
                            if let TermParts::Lit(Lit {
                                kind: LitKind::Str(s),
                            }) = values.first().unwrap()
                            {
                                Ok((s.clone(), value.get_step_data().unwrap().clone()))
                            } else {
                                Err(eyre!("Invalid key in map: {:?}", values))
                            }
                        }
                        _ => Err(eyre!("Invalid key/value in map: {:?} - {:?}", key, value)),
                    }
                })
                .collect::<Result<HashMap<_, _>>>()?;

            Ok(PatuiStepData::Map(map))
        }
        LitKind::Set(vec) => Ok(PatuiStepData::Set(
            vec.iter()
                .map(|lit| eval(lit, results))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|result| result.get_step_data().unwrap().clone())
                .collect(),
        )),
        LitKind::Wildcard => todo!(),
        LitKind::Range(_, _) => todo!(),
    }
}

fn eval_step_data(
    step_data: PatuiStepData,
    term_parts: Option<&[TermParts]>,
) -> Result<PatuiStepData> {
    tracing::trace!("Evaluating step data {:?}", step_data);

    let mut ret = step_data;

    if let Some(p) = term_parts {
        for part in p {
            tracing::trace!("Evaluating part {:?}", part);
            match part {
                TermParts::Index(p) => match &**p {
                    Expr {
                        kind: ExprKind::Term(Term { values }),
                    } => match &values[..] {
                        [TermParts::Lit(Lit {
                            kind: LitKind::Integer(i),
                        })] => {
                            let i = i.parse::<usize>().unwrap();
                            match ret {
                                PatuiStepData::Array(vec) => {
                                    ret = vec.get(i).unwrap().clone();
                                }
                                PatuiStepData::Set(vec) => {
                                    ret = vec.get(i).unwrap().clone();
                                }
                                _ => todo!(),
                            }
                        }
                        [TermParts::Lit(Lit {
                            kind: LitKind::Str(s),
                        })] => match ret {
                            PatuiStepData::Map(map) => {
                                ret = map.get(s).unwrap().clone();
                            }
                            _ => todo!(),
                        },
                        _ => todo!(),
                    },
                    _ => todo!(),
                },
                TermParts::Ident(i) => match ret {
                    PatuiStepData::Map(map) => {
                        ret = map.get(i).unwrap().clone();
                    }
                    _ => todo!(),
                },
                _ => todo!(),
            }
            tracing::trace!("Evaluated to {:?}", ret);
        }
    }

    Ok(ret)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use assertor::*;
    use bytes::Bytes;
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[test]
    fn test_simple_identity() {
        for eval in [
            PatuiStepData::String("foo".to_string()),
            PatuiStepData::Bytes(Bytes::from("foo")),
            PatuiStepData::Array(vec![
                PatuiStepData::Integer("1".to_string()),
                PatuiStepData::Integer("2".to_string()),
                PatuiStepData::Integer("3".to_string()),
            ]),
        ] {
            let result = eval_step_data(eval.clone(), None);
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(eval);
        }
    }

    #[traced_test]
    #[test]
    fn test_simple_array_lookup() {
        for (eval, lookup, expected) in [
            (
                PatuiStepData::Array(vec![
                    PatuiStepData::Integer("1".to_string()),
                    PatuiStepData::Integer("2".to_string()),
                    PatuiStepData::Integer("3".to_string()),
                ]),
                &[TermParts::Index(P {
                    ptr: Box::new(Expr {
                        kind: ExprKind::Term(Term {
                            values: vec![TermParts::Lit(Lit {
                                kind: LitKind::Integer("1".to_string()),
                            })],
                        }),
                    }),
                })],
                PatuiStepData::Integer("2".to_string()),
            ),
            (
                PatuiStepData::Array(vec![
                    PatuiStepData::Integer("1".to_string()),
                    PatuiStepData::Integer("2".to_string()),
                    PatuiStepData::String("3".to_string()),
                ]),
                &[TermParts::Index(P {
                    ptr: Box::new(Expr {
                        kind: ExprKind::Term(Term {
                            values: vec![TermParts::Lit(Lit {
                                kind: LitKind::Integer("2".to_string()),
                            })],
                        }),
                    }),
                })],
                PatuiStepData::String("3".to_string()),
            ),
        ] {
            let result = eval_step_data(eval.clone(), Some(lookup));
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn test_simple_map_lookup() {
        for (eval, lookup, expected) in [
            (
                PatuiStepData::Map(HashMap::from([
                    ("a".to_string(), PatuiStepData::Integer("1".to_string())),
                    ("b".to_string(), PatuiStepData::Integer("2".to_string())),
                    ("c".to_string(), PatuiStepData::Integer("3".to_string())),
                ])),
                &[TermParts::Index(P {
                    ptr: Box::new(Expr {
                        kind: ExprKind::Term(Term {
                            values: vec![TermParts::Lit(Lit {
                                kind: LitKind::Str("b".to_string()),
                            })],
                        }),
                    }),
                })],
                PatuiStepData::Integer("2".to_string()),
            ),
            (
                PatuiStepData::Map(HashMap::from([
                    ("a".to_string(), PatuiStepData::Integer("1".to_string())),
                    ("b".to_string(), PatuiStepData::Integer("2".to_string())),
                    ("c".to_string(), PatuiStepData::Integer("3".to_string())),
                ])),
                &[TermParts::Index(P {
                    ptr: Box::new(Expr {
                        kind: ExprKind::Term(Term {
                            values: vec![TermParts::Lit(Lit {
                                kind: LitKind::Str("c".to_string()),
                            })],
                        }),
                    }),
                })],
                PatuiStepData::Integer("3".to_string()),
            ),
        ] {
            let result = eval_step_data(eval.clone(), Some(lookup));
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn evaluate_lits() {
        for (expr_str, expected) in [
            ("true", PatuiStepData::Bool(true)),
            ("false", PatuiStepData::Bool(false)),
            ("null", PatuiStepData::Null),
            ("b[1,2,3]", PatuiStepData::Bytes(Bytes::from(vec![1, 2, 3]))),
            ("\"hello\"", PatuiStepData::String("hello".to_string())),
            ("123", PatuiStepData::Integer("123".to_string())),
            ("123.456", PatuiStepData::Float("123.456".to_string())),
            (
                "[1,2,3]",
                PatuiStepData::Array(vec![
                    PatuiStepData::Integer("1".to_string()),
                    PatuiStepData::Integer("2".to_string()),
                    PatuiStepData::Integer("3".to_string()),
                ]),
            ),
            (
                "{\"a\": 1, \"b\": 2}",
                PatuiStepData::Map(HashMap::from([
                    ("a".to_string(), PatuiStepData::Integer("1".to_string())),
                    ("b".to_string(), PatuiStepData::Integer("2".to_string())),
                ])),
            ),
            (
                "{1,2,3}",
                PatuiStepData::Set(vec![
                    PatuiStepData::Integer("1".to_string()),
                    PatuiStepData::Integer("2".to_string()),
                    PatuiStepData::Integer("3".to_string()),
                ]),
            ),
        ] {
            let expr: PatuiExpr = expr_str.try_into().unwrap();

            let ret = super::eval(&expr.expr, &HashMap::from([]));

            assert_that!(ret).is_ok();
            let ret = ret.unwrap();
            let ret = ret.get_step_data();
            assert_that!(ret).is_ok();
            assert_that!(ret.unwrap()).is_equal_to(&expected);
        }
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
            &HashMap::from([(key.expr, vec![PatuiStepData::Null])]),
        );

        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        let ret = ret.get_step_data();
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap()).is_equal_to(&PatuiStepData::Null);
    }

    #[traced_test]
    #[test]
    fn evaluate_term_equals_null_is_true() {
        let expr: PatuiExpr = "steps.test_input.out[0] == null".try_into().unwrap();

        let key: PatuiExpr = "steps.test_input.out".try_into().unwrap();

        let ret = super::eval(
            &expr.expr,
            &HashMap::from([(key.expr, vec![PatuiStepData::Null])]),
        );

        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        let ret = ret.get_step_data();
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap()).is_equal_to(&PatuiStepData::Bool(true));
    }

    #[traced_test]
    #[test]
    fn evaluate_term_equals_integer_is_true() {
        let expr: PatuiExpr = "steps.test_input.out[0] == 123".try_into().unwrap();

        let key: PatuiExpr = "steps.test_input.out".try_into().unwrap();

        let ret = super::eval(
            &expr.expr,
            &HashMap::from([(key.expr, vec![PatuiStepData::Integer("123".to_string())])]),
        );

        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        let ret = ret.get_step_data();
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap()).is_equal_to(&PatuiStepData::Bool(true));
    }

    #[traced_test]
    #[test]
    fn evaluate_term_with_indexes_equals_list_is_true() {
        let expr: PatuiExpr = "steps.test_input.out[0][1] == [1,2,3][1]"
            .try_into()
            .unwrap();

        let key: PatuiExpr = "steps.test_input.out".try_into().unwrap();

        let ret = super::eval(
            &expr.expr,
            &HashMap::from([(
                key.expr,
                vec![PatuiStepData::Array(vec![
                    PatuiStepData::Integer("0".to_string()),
                    PatuiStepData::Integer("2".to_string()),
                    PatuiStepData::Integer("4".to_string()),
                ])],
            )]),
        );

        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        let ret = ret.get_step_data();
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap()).is_equal_to(&PatuiStepData::Bool(true));
    }

    #[traced_test]
    #[test]
    fn evaluate_term_with_map_indexes_equals_map_is_true() {
        let expr: PatuiExpr = "steps.test_input.out[0][\"abc\"].def[1] == [1,2,3]"
            .try_into()
            .unwrap();

        let key: PatuiExpr = "steps.test_input.out".try_into().unwrap();

        let ret = super::eval(
            &expr.expr,
            &HashMap::from([(
                key.expr,
                vec![PatuiStepData::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiStepData::Map(HashMap::from([(
                        "def".to_string(),
                        PatuiStepData::Array(vec![
                            PatuiStepData::Integer("1".to_string()),
                            PatuiStepData::Array(vec![
                                PatuiStepData::Integer("1".to_string()),
                                PatuiStepData::Integer("2".to_string()),
                                PatuiStepData::Integer("3".to_string()),
                            ]),
                        ]),
                    )])),
                )]))],
            )]),
        );

        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        let ret = ret.get_step_data();
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap()).is_equal_to(&PatuiStepData::Bool(true));
    }
}
