use eyre::Result;

use crate::types::{expr::ast::*, PatuiStepDataFlavour};

pub(crate) fn eval_step_data(
    step_data: PatuiStepDataFlavour,
    term_parts: Option<&[TermParts]>,
) -> Result<PatuiStepDataFlavour> {
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
                                PatuiStepDataFlavour::Array(vec) => {
                                    ret = vec.get(i).unwrap().clone();
                                }
                                PatuiStepDataFlavour::Set(vec) => {
                                    ret = vec.get(i).unwrap().clone();
                                }
                                _ => todo!(),
                            }
                        }
                        [TermParts::Lit(Lit {
                            kind: LitKind::Str(s),
                        })] => {
                            let s = s.clone();
                            match ret {
                                PatuiStepDataFlavour::Map(map) => {
                                    ret = map.get(&s).unwrap().clone();
                                }
                                _ => todo!(),
                            }
                        }
                        _ => todo!(),
                    },
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
            PatuiStepDataFlavour::String("foo".to_string()),
            PatuiStepDataFlavour::Bytes(Bytes::from("foo")),
            PatuiStepDataFlavour::Array(vec![
                PatuiStepDataFlavour::Integer("1".to_string()),
                PatuiStepDataFlavour::Integer("2".to_string()),
                PatuiStepDataFlavour::Integer("3".to_string()),
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
                PatuiStepDataFlavour::Array(vec![
                    PatuiStepDataFlavour::Integer("1".to_string()),
                    PatuiStepDataFlavour::Integer("2".to_string()),
                    PatuiStepDataFlavour::Integer("3".to_string()),
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
                PatuiStepDataFlavour::Integer("2".to_string()),
            ),
            (
                PatuiStepDataFlavour::Array(vec![
                    PatuiStepDataFlavour::Integer("1".to_string()),
                    PatuiStepDataFlavour::Integer("2".to_string()),
                    PatuiStepDataFlavour::String("3".to_string()),
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
                PatuiStepDataFlavour::String("3".to_string()),
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
                PatuiStepDataFlavour::Map(HashMap::from([
                    (
                        "a".to_string(),
                        PatuiStepDataFlavour::Integer("1".to_string()),
                    ),
                    (
                        "b".to_string(),
                        PatuiStepDataFlavour::Integer("2".to_string()),
                    ),
                    (
                        "c".to_string(),
                        PatuiStepDataFlavour::Integer("3".to_string()),
                    ),
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
                PatuiStepDataFlavour::Integer("2".to_string()),
            ),
            (
                PatuiStepDataFlavour::Map(HashMap::from([
                    (
                        "a".to_string(),
                        PatuiStepDataFlavour::Integer("1".to_string()),
                    ),
                    (
                        "b".to_string(),
                        PatuiStepDataFlavour::Integer("2".to_string()),
                    ),
                    (
                        "c".to_string(),
                        PatuiStepDataFlavour::Integer("3".to_string()),
                    ),
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
                PatuiStepDataFlavour::Integer("3".to_string()),
            ),
        ] {
            let result = eval_step_data(eval.clone(), Some(lookup));
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }
}
