use std::collections::HashMap;

use bytes::Bytes;
use num::{integer::mod_floor, ToPrimitive};
use thiserror::Error;

use super::{
    ast::*,
    data::{PatuiDataError, PatuiDataInner, PatuiDataListMethods},
    PatuiData,
};

#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum EvalError {
    #[error("Index is not a valid size, cannot convert to signed 64-bit integer")]
    BadIndexSize,
    #[error("Indexing type '{0}' unsupported for type '{1}'")]
    IndexUnsupported(String, String),
    #[error("Indexing type '{0}' unsupported")]
    IndexTypeUnsupported(String),
    #[error("Index in first part of term not valid")]
    IndexNoLit,
    #[error("Index range is not valid")]
    BadIndexRange,
    #[error("Invalid literal for evaluating {0}")]
    LitEvalUnsupported(String),
    #[error("Invalid function call, no method supplied")]
    CallNoMethodFound,
    #[error("Invalid function call, method {0} not found on type '{1}'")]
    CallMethodNotFound(String, String),
    #[error("Invalid term")]
    InvalidTerm,
    #[error("Invalid results, must be a map")]
    ResultsNotMap,
    #[error("Data not found")]
    DataNotFound,
    #[error("Invalid data {0}")]
    InvalidDataInner(#[from] PatuiDataError),
    #[error("Invalid data type {0} when we expected {1}")]
    InvalidDataType(String, String),
}

/// Evaluate the expression against some known results.
pub fn eval(expr: &Expr, results: &PatuiData) -> Result<PatuiData, EvalError> {
    tracing::trace!("Evaluating: {:?}", expr);
    tracing::trace!("Results: {:?}", results);

    match expr {
        Expr::Term(term_parts) => eval_term(term_parts, results),
        Expr::UnOp(_, _) => todo!(),
        Expr::BinOp(op, lhs, rhs) => eval_binop(op, lhs, rhs, results),
        Expr::If(_, _, _) => todo!(),
    }
}

fn eval_term(term_parts: &[TermPart], results: &PatuiData) -> Result<PatuiData, EvalError> {
    let mut data = None;

    for term_part in term_parts {
        tracing::trace!("Term Part: {:?}", term_part);
        match term_part {
            TermPart::Lit(lit) => data = Some(eval_lit(lit, results)?),
            TermPart::Ident(ident) => match results {
                PatuiData::Known(PatuiDataInner::Map(hash_map)) => {
                    let key = ident.clone();
                    tracing::trace!("Ident key: {:?}", key);
                    match data.as_ref() {
                        Some(existing) => {
                            tracing::trace!("Existing ident lookup: {:?}", existing);
                            let existing_inner = match existing {
                                PatuiData::Known(inner) => inner,
                                PatuiData::Pending(inner) => inner,
                                PatuiData::Unknown => return Ok(PatuiData::Unknown),
                            };
                            let value = match existing_inner {
                                PatuiDataInner::Null => todo!(),
                                PatuiDataInner::Bool(_) => todo!(),
                                PatuiDataInner::Bytes(_) => todo!(),
                                PatuiDataInner::String(_) => todo!(),
                                PatuiDataInner::Integer(_) => todo!(),
                                PatuiDataInner::Decimal(_) => todo!(),
                                PatuiDataInner::List(_) => todo!(),
                                PatuiDataInner::Map(map) => {
                                    map.get(&key).ok_or(EvalError::DataNotFound)?.clone()
                                }
                                PatuiDataInner::Set(_) => todo!(),
                            };
                            data = Some(value.clone());
                        }
                        None => {
                            let value = hash_map.get(&key).ok_or(EvalError::DataNotFound)?;
                            data = Some(value.clone());
                        }
                    }
                }
                _ => return Err(EvalError::ResultsNotMap),
            },
            TermPart::Index(index) => match data {
                Some(existing) => data = Some(eval_index(&existing, index, results)?),
                None => return Err(EvalError::IndexNoLit),
            },
            TermPart::Call(function_name, args) => match data {
                Some(existing) => data = Some(eval_call(&existing, function_name, args, results)?),
                None => return Err(EvalError::CallNoMethodFound),
            },
        }
    }

    data.ok_or(EvalError::InvalidTerm)
}

fn eval_binop(
    op: &BinOp,
    lhs: &Expr,
    rhs: &Expr,
    results: &PatuiData,
) -> Result<PatuiData, EvalError> {
    let lhs = eval(lhs, results)?;
    let rhs = eval(rhs, results)?;

    let mut known = true;
    let lhs_inner = match lhs {
        PatuiData::Known(inner) => inner,
        PatuiData::Pending(inner) => {
            known = false;
            inner
        }
        PatuiData::Unknown => return Ok(PatuiData::Unknown),
    };
    let rhs_inner = match rhs {
        PatuiData::Known(inner) => inner,
        PatuiData::Pending(inner) => {
            known = false;
            inner
        }
        PatuiData::Unknown => return Ok(PatuiData::Unknown),
    };

    match op {
        BinOp::Add => todo!(),
        BinOp::Subtract => todo!(),
        BinOp::Multiply => todo!(),
        BinOp::Divide => todo!(),
        BinOp::Modulo => todo!(),
        BinOp::And | BinOp::Or => {
            let PatuiDataInner::Bool(lhs_bool) = lhs_inner else {
                return Err(EvalError::InvalidDataType(
                    format!("{}", lhs_inner),
                    "Bool".to_string(),
                ));
            };

            let PatuiDataInner::Bool(rhs_bool) = rhs_inner else {
                return Err(EvalError::InvalidDataType(
                    format!("{}", rhs_inner),
                    "Bool".to_string(),
                ));
            };

            let res = if op == &BinOp::And {
                lhs_bool && rhs_bool
            } else {
                lhs_bool || rhs_bool
            };

            Ok(if known {
                PatuiData::Known(PatuiDataInner::Bool(res))
            } else {
                PatuiData::Pending(PatuiDataInner::Bool(res))
            })
        }
        BinOp::Equal | BinOp::NotEqual => {
            let res = if op == &BinOp::Equal {
                lhs_inner == rhs_inner
            } else {
                lhs_inner != rhs_inner
            };

            Ok(if known {
                PatuiData::Known(PatuiDataInner::Bool(res))
            } else {
                PatuiData::Pending(PatuiDataInner::Bool(res))
            })
        }
        BinOp::LessThan => todo!(),
        BinOp::LessThanEqual => todo!(),
        BinOp::GreaterThan => todo!(),
        BinOp::GreaterThanEqual => todo!(),
        BinOp::Contains => todo!(),
        BinOp::NotContains => todo!(),
    }
}

fn eval_lit(lit: &Lit, results: &PatuiData) -> Result<PatuiData, EvalError> {
    match lit {
        Lit::Null => Ok(PatuiData::Known(PatuiDataInner::Null)),
        Lit::Bool(b) => Ok(PatuiData::Known(PatuiDataInner::Bool(*b))),
        Lit::Bytes(bytes) => Ok(PatuiData::Known(PatuiDataInner::Bytes(bytes.clone()))),
        Lit::Integer(integer) => Ok(PatuiData::Known(PatuiDataInner::Integer(integer.clone()))),
        Lit::Decimal(float) => Ok(PatuiData::Known(PatuiDataInner::Decimal(float.clone()))),
        Lit::String(s) => Ok(PatuiData::Known(PatuiDataInner::String(s.clone()))),
        Lit::List(vec) => Ok(PatuiData::Known(PatuiDataInner::List(
            vec.iter()
                .map(|expr| eval(expr, results))
                .collect::<Result<Vec<PatuiData>, EvalError>>()?,
        ))),
        Lit::Map(vec) => Ok(PatuiData::Known(PatuiDataInner::Map(
            vec.iter()
                .map(|(key, expr)| {
                    let value = eval(expr, results)?;
                    Ok((key.clone(), value))
                })
                .collect::<Result<HashMap<_, _>, EvalError>>()?,
        ))),
        Lit::Set(vec) => Ok(PatuiData::Known(PatuiDataInner::Set(
            vec.iter()
                .map(|lit| eval(lit, results))
                .collect::<Result<Vec<_>, EvalError>>()?,
        ))),
        Lit::Wildcard => Err(EvalError::LitEvalUnsupported("Wildcard".to_string())),
        Lit::Range(i1, i2) => Ok(PatuiData::Known(PatuiDataInner::List(vec![
            eval(i1, results)?,
            eval(i2, results)?,
        ]))),
    }
}

fn eval_index(base: &PatuiData, index: &Expr, results: &PatuiData) -> Result<PatuiData, EvalError> {
    tracing::trace!("Index: {:?}", index);
    tracing::trace!("Base: {:?}", base);

    let index = eval(index, results)?;
    let mut known = true;

    let index = match index {
        PatuiData::Known(inner) => inner,
        PatuiData::Pending(inner) => {
            known = false;
            inner
        }
        PatuiData::Unknown => return Ok(PatuiData::Unknown),
    };

    let base_inner = match base {
        PatuiData::Known(inner) => inner,
        PatuiData::Pending(inner) => {
            known = false;
            inner
        }
        PatuiData::Unknown => return Ok(PatuiData::Unknown),
    };

    tracing::trace!("Evaluated Index: {:?}", index);
    tracing::trace!("Evaluated Base inner: {:?}", base_inner);

    match index {
        PatuiDataInner::String(s) => match base_inner {
            PatuiDataInner::Map(hash_map) => {
                let key = s.clone();
                let value = hash_map.get(&key).ok_or(EvalError::DataNotFound)?;
                match value {
                    PatuiData::Known(inner) if !known => Ok(PatuiData::Pending(inner.clone())),
                    _ => Ok(value.clone()),
                }
            }

            PatuiDataInner::Null => Err(EvalError::IndexUnsupported(
                "String".to_string(),
                "Null".to_string(),
            )),
            PatuiDataInner::Bool(_) => Err(EvalError::IndexUnsupported(
                "String".to_string(),
                "Bool".to_string(),
            )),
            PatuiDataInner::Bytes(_) => Err(EvalError::IndexUnsupported(
                "String".to_string(),
                "Bytes".to_string(),
            )),
            PatuiDataInner::String(_) => Err(EvalError::IndexUnsupported(
                "String".to_string(),
                "String".to_string(),
            )),
            PatuiDataInner::Integer(_) => Err(EvalError::IndexUnsupported(
                "String".to_string(),
                "Integer".to_string(),
            )),
            PatuiDataInner::Decimal(_) => Err(EvalError::IndexUnsupported(
                "String".to_string(),
                "Decimal".to_string(),
            )),
            PatuiDataInner::List(_) => Err(EvalError::IndexUnsupported(
                "String".to_string(),
                "List".to_string(),
            )),
            PatuiDataInner::Set(_) => Err(EvalError::IndexUnsupported(
                "String".to_string(),
                "Set".to_string(),
            )),
        },
        PatuiDataInner::Integer(integer) => {
            let index = integer.to_isize().ok_or(EvalError::BadIndexSize)?;
            match base_inner {
                PatuiDataInner::List(vec) => {
                    if index > vec.len() as isize {
                        if !known {
                            return Ok(PatuiData::Unknown);
                        } else {
                            return Err(EvalError::DataNotFound);
                        }
                    }
                    // mod_floor is used to handle negative indices, indices above the length are
                    // handled above.
                    let index = mod_floor(index, vec.len() as isize) as usize;
                    Ok(vec[index].clone())
                }
                PatuiDataInner::Bytes(bytes) => {
                    let index = mod_floor(index, bytes.len() as isize) as usize;
                    Ok(if known {
                        PatuiData::Known(PatuiDataInner::Integer(bytes[index].into()))
                    } else {
                        PatuiData::Pending(PatuiDataInner::Integer(bytes[index].into()))
                    })
                }
                PatuiDataInner::String(s) => {
                    let index = mod_floor(index, s.len() as isize) as usize;
                    Ok(if known {
                        PatuiData::Known(PatuiDataInner::String(
                            s.chars().nth(index).unwrap().to_string(),
                        ))
                    } else {
                        PatuiData::Pending(PatuiDataInner::String(
                            s.chars().nth(index).unwrap().to_string(),
                        ))
                    })
                }

                PatuiDataInner::Map(_) => Err(EvalError::IndexUnsupported(
                    "Integer".to_string(),
                    "Map".to_string(),
                )),
                PatuiDataInner::Set(_) => Err(EvalError::IndexUnsupported(
                    "Integer".to_string(),
                    "Set".to_string(),
                )),
                PatuiDataInner::Null => Err(EvalError::IndexUnsupported(
                    "Integer".to_string(),
                    "Null".to_string(),
                )),
                PatuiDataInner::Bool(_) => Err(EvalError::IndexUnsupported(
                    "Integer".to_string(),
                    "Bool".to_string(),
                )),
                PatuiDataInner::Integer(_) => Err(EvalError::IndexUnsupported(
                    "Integer".to_string(),
                    "Integer".to_string(),
                )),
                PatuiDataInner::Decimal(_) => Err(EvalError::IndexUnsupported(
                    "Integer".to_string(),
                    "Decimal".to_string(),
                )),
            }
        }
        PatuiDataInner::List(vec) => {
            tracing::trace!("Vec: {:?}", vec);
            let Some(i1) = vec.first() else {
                tracing::trace!("No first elements in list index");
                return Err(EvalError::BadIndexRange);
            };
            let i1 = match i1 {
                PatuiData::Known(inner) => inner,
                PatuiData::Pending(inner) => {
                    known = false;
                    inner
                }
                PatuiData::Unknown => return Ok(PatuiData::Unknown),
            };
            let i1 = match i1 {
                PatuiDataInner::Integer(i) => i.to_isize().ok_or(EvalError::BadIndexSize)?,
                _ => return Err(EvalError::BadIndexRange),
            };

            let Some(i2) = vec.get(1) else {
                tracing::trace!("No second elements in list index");
                return Err(EvalError::BadIndexRange);
            };
            let i2 = match i2 {
                PatuiData::Known(inner) => inner,
                PatuiData::Pending(inner) => {
                    known = false;
                    inner
                }
                PatuiData::Unknown => return Ok(PatuiData::Unknown),
            };
            let i2 = match i2 {
                PatuiDataInner::Integer(i) => i.to_isize().ok_or(EvalError::BadIndexSize)?,
                _ => return Err(EvalError::BadIndexRange),
            };

            if vec.get(2).is_some() {
                tracing::trace!("Got a third element in list index");
                return Err(EvalError::BadIndexRange);
            }

            match base_inner {
                PatuiDataInner::String(s) => {
                    let i1 = mod_floor(i1, s.len() as isize) as usize;
                    let i2 = mod_floor(i2, s.len() as isize) as usize;
                    Ok(if known {
                        PatuiData::Known(PatuiDataInner::String(s[i1..i2].to_string()))
                    } else {
                        PatuiData::Pending(PatuiDataInner::String(s[i1..i2].to_string()))
                    })
                }
                PatuiDataInner::Bytes(bytes) => {
                    let i1 = mod_floor(i1, bytes.len() as isize) as usize;
                    let i2 = mod_floor(i2, bytes.len() as isize) as usize;
                    let bytes = bytes[i1..i2].to_vec();
                    Ok(if known {
                        PatuiData::Known(PatuiDataInner::Bytes(Bytes::from(bytes)))
                    } else {
                        PatuiData::Pending(PatuiDataInner::Bytes(Bytes::from(bytes)))
                    })
                }
                PatuiDataInner::List(vec) => {
                    let i1 = mod_floor(i1, vec.len() as isize) as usize;
                    let i2 = mod_floor(i2, vec.len() as isize) as usize;
                    let vec = vec[i1..i2].to_vec();
                    Ok(if known {
                        PatuiData::Known(PatuiDataInner::List(vec))
                    } else {
                        PatuiData::Pending(PatuiDataInner::List(vec))
                    })
                }

                PatuiDataInner::Null => Err(EvalError::IndexUnsupported(
                    "List".to_string(),
                    "Null".to_string(),
                )),
                PatuiDataInner::Bool(_) => Err(EvalError::IndexUnsupported(
                    "List".to_string(),
                    "Bool".to_string(),
                )),
                PatuiDataInner::Integer(_) => Err(EvalError::IndexUnsupported(
                    "List".to_string(),
                    "Integer".to_string(),
                )),
                PatuiDataInner::Decimal(_) => Err(EvalError::IndexUnsupported(
                    "List".to_string(),
                    "Decimal".to_string(),
                )),
                PatuiDataInner::Map(_) => Err(EvalError::IndexUnsupported(
                    "List".to_string(),
                    "Map".to_string(),
                )),
                PatuiDataInner::Set(_) => Err(EvalError::IndexUnsupported(
                    "List".to_string(),
                    "Set".to_string(),
                )),
            }
        }
        PatuiDataInner::Null => Err(EvalError::IndexTypeUnsupported("Null".to_string())),
        PatuiDataInner::Bool(_) => Err(EvalError::IndexTypeUnsupported("Bool".to_string())),
        PatuiDataInner::Bytes(_) => Err(EvalError::IndexTypeUnsupported("Bytes".to_string())),
        PatuiDataInner::Decimal(_) => Err(EvalError::IndexTypeUnsupported("Decimal".to_string())),
        PatuiDataInner::Map(_) => Err(EvalError::IndexTypeUnsupported("Map".to_string())),
        PatuiDataInner::Set(_) => Err(EvalError::IndexTypeUnsupported("Set".to_string())),
    }
}

fn eval_call(
    existing: &PatuiData,
    function_name: &str,
    args: &[Expr],
    results: &PatuiData,
) -> Result<PatuiData, EvalError> {
    let (known, existing) = match existing {
        PatuiData::Known(inner) => (true, inner),
        PatuiData::Pending(inner) => (false, inner),
        PatuiData::Unknown => return Ok(PatuiData::Unknown),
    };

    match existing {
        PatuiDataInner::Null => todo!(),
        PatuiDataInner::Bool(_) => todo!(),
        PatuiDataInner::Bytes(_) => todo!(),
        PatuiDataInner::String(_) => todo!(),
        PatuiDataInner::Integer(_) => todo!(),
        PatuiDataInner::Decimal(_) => todo!(),
        PatuiDataInner::List(vec) => eval_list_call(known, vec, function_name, args, results),
        PatuiDataInner::Map(_) => todo!(),
        PatuiDataInner::Set(_) => todo!(),
    }
}

fn eval_list_call(
    known: bool,
    vec: &[PatuiData],
    function_name: &str,
    args: &[Expr],
    _results: &PatuiData,
) -> Result<PatuiData, EvalError> {
    let list_functions = PatuiDataListMethods::new(vec, known);
    match function_name {
        "len" => Ok(list_functions.len(args)?),
        _ => Err(EvalError::CallMethodNotFound(
            function_name.to_string(),
            "List".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use bytes::Bytes;
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[test]
    fn simple_identity() {
        for (eval_data, expected) in [
            (
                Expr::Term(vec![TermPart::Lit(Lit::Null)]),
                PatuiData::Known(PatuiDataInner::Null),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::Bool(true))]),
                PatuiData::Known(PatuiDataInner::Bool(true)),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::Bytes(Bytes::from("hello")))]),
                PatuiData::Known(PatuiDataInner::Bytes(Bytes::from("hello"))),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::Integer(42))]),
                PatuiData::Known(PatuiDataInner::Integer(42)),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::Decimal(123.45))]),
                PatuiData::Known(PatuiDataInner::Decimal(123.45)),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::String("hello".to_string()))]),
                PatuiData::Known(PatuiDataInner::String("hello".to_string())),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                    Expr::Term(vec![TermPart::Lit(Lit::String("hello".to_string()))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(42))]),
                    Expr::Term(vec![TermPart::Lit(Lit::String("hello".to_string()))]),
                ]))]),
                PatuiData::Known(PatuiDataInner::List(vec![
                    PatuiData::Known(PatuiDataInner::String("hello".to_string())),
                    PatuiData::Known(PatuiDataInner::Integer(42)),
                    PatuiData::Known(PatuiDataInner::String("hello".to_string())),
                ])),
            ),
        ] {
            let result = eval(
                &eval_data,
                &PatuiData::Known(PatuiDataInner::Map(HashMap::new())),
            );
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn simple_lit_index() {
        for (eval_data, expected) in [
            (
                Expr::Term(vec![
                    TermPart::Lit(Lit::List(vec![
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::String("3".to_string()))]),
                    ])),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Integer(2.into())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Lit(Lit::List(vec![
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::String("3".to_string()))]),
                    ])),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        (-1).into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::String("3".to_string())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Lit(Lit::Map(vec![
                        (
                            "abc".to_string(),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                        ),
                        (
                            "def".to_string(),
                            Expr::Term(vec![TermPart::Lit(Lit::Map(vec![(
                                "ghi".to_string(),
                                Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                            )]))]),
                        ),
                    ])),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "abc".to_string(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Integer(1.into())),
            ),
        ] {
            let result = eval(
                &eval_data,
                &PatuiData::Known(PatuiDataInner::Map(HashMap::new())),
            );
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn simple_errors() {
        for (eval_data, expected) in [
            (Expr::Term(vec![]), EvalError::InvalidTerm),
            (
                Expr::Term(vec![TermPart::Index(Box::new(Expr::Term(vec![
                    TermPart::Lit(Lit::Integer(1.into())),
                ])))]),
                EvalError::IndexNoLit,
            ),
        ] {
            let result = eval(
                &eval_data,
                &PatuiData::Known(PatuiDataInner::Map(HashMap::new())),
            );
            assert_that!(result).is_err();
            assert_that!(result.unwrap_err()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn known_results() {
        for (eval_data, lookup, expected) in [
            (
                Expr::Term(vec![TermPart::Ident("abc".to_string())]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    (
                        "abc".to_string(),
                        PatuiData::Known(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Known(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    ),
                    (
                        "def".to_string(),
                        PatuiData::Known(PatuiDataInner::Integer(4.into())),
                    ),
                ]))),
                PatuiData::Known(PatuiDataInner::List(vec![
                    PatuiData::Known(PatuiDataInner::Integer(1.into())),
                    PatuiData::Known(PatuiDataInner::Integer(2.into())),
                    PatuiData::Known(PatuiDataInner::Integer(3.into())),
                ])),
            ),
            (
                Expr::Term(vec![TermPart::Ident("def".to_string())]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    (
                        "abc".to_string(),
                        PatuiData::Known(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Known(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    ),
                    (
                        "def".to_string(),
                        PatuiData::Known(PatuiDataInner::Integer(4.into())),
                    ),
                ]))),
                PatuiData::Known(PatuiDataInner::Integer(4.into())),
            ),
        ] {
            let result = eval(&eval_data, &lookup);
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn known_results_indexing() {
        for (eval_data, lookup, expected) in [
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    (
                        "abc".to_string(),
                        PatuiData::Known(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Known(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    ),
                    (
                        "def".to_string(),
                        PatuiData::Known(PatuiDataInner::Integer(4.into())),
                    ),
                ]))),
                PatuiData::Known(PatuiDataInner::Integer(2.into())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "def".to_string(),
                    ))]))),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        0.into(),
                    ))]))),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "ghi".to_string(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                        "def".to_string(),
                        PatuiData::Known(PatuiDataInner::List(vec![PatuiData::Known(
                            PatuiDataInner::Map(HashMap::from([(
                                "ghi".to_string(),
                                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                                    "abc".to_string(),
                                    PatuiData::Known(PatuiDataInner::List(vec![
                                        PatuiData::Known(PatuiDataInner::Integer(1.into())),
                                        PatuiData::Known(PatuiDataInner::Integer(2.into())),
                                        PatuiData::Known(PatuiDataInner::Integer(3.into())),
                                    ])),
                                )]))),
                            )])),
                        )])),
                    )]))),
                )]))),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::List(vec![
                        PatuiData::Known(PatuiDataInner::Integer(1.into())),
                        PatuiData::Known(PatuiDataInner::Integer(2.into())),
                        PatuiData::Known(PatuiDataInner::Integer(3.into())),
                    ])),
                )]))),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Ident("def".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![
                        TermPart::Ident("def".to_string()),
                        TermPart::Ident("abc".to_string()),
                    ]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    (
                        "abc".to_string(),
                        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                            "def".to_string(),
                            PatuiData::Known(PatuiDataInner::List(vec![
                                PatuiData::Known(PatuiDataInner::Integer(1.into())),
                                PatuiData::Known(PatuiDataInner::Integer(2.into())),
                                PatuiData::Known(PatuiDataInner::Integer(3.into())),
                            ])),
                        )]))),
                    ),
                    (
                        "def".to_string(),
                        PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                            "abc".to_string(),
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                        )]))),
                    ),
                ]))),
                PatuiData::Known(PatuiDataInner::Integer(2.into())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "def".to_string(),
                    ))]))),
                    TermPart::Call("len".to_string(), vec![]),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                        "def".to_string(),
                        PatuiData::Known(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Known(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    )]))),
                )]))),
                PatuiData::Known(PatuiDataInner::Integer(3.into())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::String("testing".into())),
                )]))),
                PatuiData::Known(PatuiDataInner::String("e".into())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Range(
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(4.into()))])),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::String("testing".into())),
                )]))),
                PatuiData::Known(PatuiDataInner::String("est".into())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Bytes(Bytes::from("testing"))),
                )]))),
                PatuiData::Known(PatuiDataInner::Integer(101.into())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Range(
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(4.into()))])),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Bytes(Bytes::from("testing"))),
                )]))),
                PatuiData::Known(PatuiDataInner::Bytes(Bytes::from("est"))),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::List(vec![
                        PatuiData::Known(PatuiDataInner::Integer(1.into())),
                        PatuiData::Known(PatuiDataInner::Integer(2.into())),
                        PatuiData::Known(PatuiDataInner::Integer(3.into())),
                    ])),
                )]))),
                PatuiData::Known(PatuiDataInner::Integer(2.into())),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Range(
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(4.into()))])),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::List(vec![
                        PatuiData::Known(PatuiDataInner::Integer(1.into())),
                        PatuiData::Known(PatuiDataInner::Integer(2.into())),
                        PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        PatuiData::Known(PatuiDataInner::Integer(4.into())),
                        PatuiData::Known(PatuiDataInner::Integer(5.into())),
                    ])),
                )]))),
                PatuiData::Known(PatuiDataInner::List(vec![
                    PatuiData::Known(PatuiDataInner::Integer(2.into())),
                    PatuiData::Known(PatuiDataInner::Integer(3.into())),
                    PatuiData::Known(PatuiDataInner::Integer(4.into())),
                ])),
            ),
        ] {
            let result = eval(&eval_data, &lookup);
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn pending_results() {
        for (eval_data, lookup, expected) in [
            (
                Expr::Term(vec![TermPart::Ident("abc".to_string())]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    (
                        "abc".to_string(),
                        PatuiData::Pending(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Pending(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    ),
                    ("def".to_string(), PatuiData::Unknown),
                ]))),
                PatuiData::Pending(PatuiDataInner::List(vec![
                    PatuiData::Known(PatuiDataInner::Integer(1.into())),
                    PatuiData::Pending(PatuiDataInner::Integer(2.into())),
                    PatuiData::Known(PatuiDataInner::Integer(3.into())),
                ])),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "def".to_string(),
                    ))]))),
                    TermPart::Call("len".to_string(), vec![]),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                        "def".to_string(),
                        PatuiData::Pending(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Pending(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    )]))),
                )]))),
                PatuiData::Pending(PatuiDataInner::Integer(3.into())),
            ),
        ] {
            let result = eval(&eval_data, &lookup);
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn unknown_results() {
        for (eval_data, lookup, expected) in [
            (
                Expr::Term(vec![TermPart::Ident("def".to_string())]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    (
                        "abc".to_string(),
                        PatuiData::Pending(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Pending(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    ),
                    ("def".to_string(), PatuiData::Unknown),
                ]))),
                PatuiData::Unknown,
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "def".to_string(),
                    ))]))),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        5.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                        "def".to_string(),
                        PatuiData::Pending(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Known(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    )]))),
                )]))),
                PatuiData::Unknown,
            ),
        ] {
            let result = eval(&eval_data, &lookup);
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn not_found_results() {
        for (eval_data, lookup, expected) in [
            (
                Expr::Term(vec![TermPart::Ident("ghi".to_string())]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([
                    (
                        "abc".to_string(),
                        PatuiData::Known(PatuiDataInner::Integer(1.into())),
                    ),
                    (
                        "def".to_string(),
                        PatuiData::Known(PatuiDataInner::Integer(2.into())),
                    ),
                ]))),
                EvalError::DataNotFound,
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "def".to_string(),
                    ))]))),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        5.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                        "def".to_string(),
                        PatuiData::Known(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Known(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    )]))),
                )]))),
                EvalError::DataNotFound,
            ),
        ] {
            let result = eval(&eval_data, &lookup);
            assert_that!(result).is_err();
            assert_that!(result.unwrap_err()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn error_results_lookup() {
        for (eval_data, lookup, expected) in [
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                        "abc".to_string(),
                        PatuiData::Known(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::Integer(1.into())),
                            PatuiData::Known(PatuiDataInner::Integer(2.into())),
                            PatuiData::Known(PatuiDataInner::Integer(3.into())),
                        ])),
                    )]))),
                )]))),
                EvalError::IndexUnsupported("Integer".to_string(), "Map".to_string()),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Set(vec![
                        PatuiData::Known(PatuiDataInner::Integer(1.into())),
                        PatuiData::Known(PatuiDataInner::Integer(2.into())),
                        PatuiData::Known(PatuiDataInner::Integer(3.into())),
                    ])),
                )]))),
                EvalError::IndexUnsupported("Integer".to_string(), "Set".to_string()),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Null),
                )]))),
                EvalError::IndexUnsupported("Integer".to_string(), "Null".to_string()),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Bool(true)),
                )]))),
                EvalError::IndexUnsupported("Integer".to_string(), "Bool".to_string()),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(1.into())),
                )]))),
                EvalError::IndexUnsupported("Integer".to_string(), "Integer".to_string()),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(
                        1.into(),
                    ))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Decimal(123.45)),
                )]))),
                EvalError::IndexUnsupported("Integer".to_string(), "Decimal".to_string()),
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::List(vec![]))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Decimal(123.45)),
                )]))),
                EvalError::BadIndexRange,
            ),
            (
                Expr::Term(vec![
                    TermPart::Ident("abc".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(2.into()))]),
                        Expr::Term(vec![TermPart::Lit(Lit::String("3".to_string()))]),
                    ]))]))),
                ]),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "abc".to_string(),
                    PatuiData::Known(PatuiDataInner::Decimal(123.45)),
                )]))),
                EvalError::BadIndexRange,
            ),
        ] {
            let result = eval(&eval_data, &lookup);
            assert_that!(result).is_err();
            assert_that!(result.unwrap_err()).is_equal_to(expected);
        }
    }

    #[traced_test]
    #[test]
    fn simple_comparison() {
        for (eval_data, expected) in [
            (
                Expr::BinOp(
                    BinOp::Equal,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                ),
                PatuiData::Known(PatuiDataInner::Bool(true)),
            ),
            (
                Expr::BinOp(
                    BinOp::NotEqual,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1.into()))])),
                ),
                PatuiData::Known(PatuiDataInner::Bool(false)),
            ),
            (
                Expr::BinOp(
                    BinOp::And,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                ),
                PatuiData::Known(PatuiDataInner::Bool(true)),
            ),
            (
                Expr::BinOp(
                    BinOp::And,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                ),
                PatuiData::Known(PatuiDataInner::Bool(false)),
            ),
            (
                Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                ),
                PatuiData::Known(PatuiDataInner::Bool(true)),
            ),
            (
                Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(false))])),
                ),
                PatuiData::Known(PatuiDataInner::Bool(false)),
            ),
        ] {
            let result = eval(
                &eval_data,
                &PatuiData::Known(PatuiDataInner::Map(HashMap::new())),
            );
            assert_that!(result).is_ok();
            assert_that!(result.unwrap()).is_equal_to(expected);
        }
    }
}
