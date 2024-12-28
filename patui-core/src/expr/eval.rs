use std::collections::HashMap;

use num::integer::mod_floor;
use thiserror::Error;

use super::{
    ast::*,
    data::{PatuiDataError, PatuiDataInner, PatuiDataListMethods},
    PatuiData,
};

#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum EvalError {
    #[error("Invalid type")]
    InvalidType,
    #[error("Invalid operation")]
    InvalidOperation,
    #[error("Index is not a valid size, cannot convert to signed 64-bit integer")]
    InvalidIndexSize,
    #[error("Index in first part of term not valid")]
    InvalidIndexNoLit,
    #[error("Invalid key, nit found in map")]
    InvalidKey,
    #[error("Invalid range")]
    InvalidRange,
    #[error("Invalid call")]
    InvalidCall,
    #[error("Invalid ident")]
    InvalidIdent,
    #[error("Invalid lit")]
    InvalidLit,
    #[error("Invalid function call, no method supplied")]
    InvalidCallNoMethod,
    #[error("Invalid function call, method {0} not found on type '{1}'")]
    InvalidCallMethod(String, String),
    #[error("Invalid unop")]
    InvalidUnOp,
    #[error("Invalid binop")]
    InvalidBinOp,
    #[error("Invalid term")]
    InvalidTerm,
    #[error("Invalid expr")]
    InvalidExpr,
    #[error("Invalid data")]
    InvalidData,
    #[error("Invalid results")]
    InvalidResults,
    #[error("Invalid data {0}")]
    InvalidDataInner(#[from] PatuiDataError),
}

/// Evaluate the expression against some known results.
pub fn eval(expr: &Expr, results: &PatuiData) -> Result<PatuiData, EvalError> {
    tracing::trace!("Evaluating: {:?}", expr);
    tracing::trace!("Results: {:?}", results);

    match expr {
        Expr::Term(term_parts) => eval_term(term_parts, results),
        Expr::UnOp(op, expr) => todo!(),
        Expr::BinOp(op, lhs, rhs) => todo!(),
        Expr::If(cond, then, else_) => todo!(),
    }
}

fn eval_term(term_parts: &[TermPart], results: &PatuiData) -> Result<PatuiData, EvalError> {
    let mut data = None;

    for term_part in term_parts {
        match term_part {
            TermPart::Lit(lit) => data = Some(eval_lit(lit, results)?),
            TermPart::Ident(ident) => match results {
                PatuiData::Known(PatuiDataInner::Map(hash_map)) => {
                    let key = ident.clone();
                    let value = hash_map.get(&key).ok_or(EvalError::InvalidKey)?;
                    data = Some(value.clone());
                }
                _ => return Err(EvalError::InvalidResults),
            },
            TermPart::Index(index) => match data {
                Some(existing) => data = Some(eval_index(&existing, index, results)?),
                None => return Err(EvalError::InvalidIndexNoLit),
            },
            TermPart::Call(function_name, args) => match data {
                Some(existing) => data = Some(eval_call(&existing, function_name, args, results)?),
                None => return Err(EvalError::InvalidCallNoMethod),
            },
        }
    }

    data.ok_or(EvalError::InvalidTerm)
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
        Lit::Wildcard => todo!(),
        Lit::Range(expr, expr1) => todo!(),
    }
}

fn eval_index(
    existing: &PatuiData,
    index: &Expr,
    results: &PatuiData,
) -> Result<PatuiData, EvalError> {
    tracing::trace!("Index: {:?}", index);
    tracing::trace!("Existing: {:?}", existing);

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

    let existing = match existing {
        PatuiData::Known(inner) => inner,
        PatuiData::Pending(inner) => {
            known = false;
            inner
        }
        PatuiData::Unknown => return Ok(PatuiData::Unknown),
    };

    match index {
        PatuiDataInner::Null => todo!(),
        PatuiDataInner::Bool(_) => todo!(),
        PatuiDataInner::Bytes(bytes) => todo!(),
        PatuiDataInner::String(s) => match existing {
            PatuiDataInner::Null => todo!(),
            PatuiDataInner::Bool(_) => todo!(),
            PatuiDataInner::Bytes(bytes) => todo!(),
            PatuiDataInner::String(_) => todo!(),
            PatuiDataInner::Integer(integer) => todo!(),
            PatuiDataInner::Decimal(float) => todo!(),
            PatuiDataInner::List(vec) => todo!(),
            PatuiDataInner::Map(hash_map) => {
                let key = s.clone();
                let value = hash_map.get(&key).ok_or(EvalError::InvalidKey)?;
                match value {
                    PatuiData::Known(inner) if !known => Ok(PatuiData::Pending(inner.clone())),
                    _ => Ok(value.clone()),
                }
            }
            PatuiDataInner::Set(vec) => todo!(),
        },
        PatuiDataInner::Integer(integer) => {
            let index = integer.to_isize().ok_or(EvalError::InvalidIndexSize)?;
            match existing {
                PatuiDataInner::List(vec) => {
                    if index > vec.len() as isize && !known {
                        return Ok(PatuiData::Unknown);
                    }
                    let index = mod_floor(index, vec.len() as isize) as usize;
                    Ok(vec[index].clone())
                }
                PatuiDataInner::Map(hash_map) => {
                    todo!();
                }
                PatuiDataInner::Set(vec) => {
                    let index = mod_floor(index, vec.len() as isize) as usize;
                    Ok(vec[index].clone())
                }
                _ => Err(EvalError::InvalidIndexNoLit),
            }
        }
        PatuiDataInner::Decimal(float) => todo!(),
        PatuiDataInner::List(vec) => todo!(),
        PatuiDataInner::Map(hash_map) => todo!(),
        PatuiDataInner::Set(vec) => todo!(),
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
        _ => Err(EvalError::InvalidCallMethod(
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
    fn simple_errors() {
        for (eval_data, expected) in [
            (Expr::Term(vec![]), EvalError::InvalidTerm),
            (Expr::Term(vec![]), EvalError::InvalidTerm),
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
                Expr::Term(vec![TermPart::Lit(Lit::Integer(rug::Integer::from(42)))]),
                PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::Decimal(rug::Float::with_val(
                    53, 123.45,
                )))]),
                PatuiData::Known(PatuiDataInner::Decimal(rug::Float::with_val(53, 123.45))),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::String("hello".to_string()))]),
                PatuiData::Known(PatuiDataInner::String("hello".to_string())),
            ),
            (
                Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                    Expr::Term(vec![TermPart::Lit(Lit::String("hello".to_string()))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(rug::Integer::from(42)))]),
                    Expr::Term(vec![TermPart::Lit(Lit::String("hello".to_string()))]),
                ]))]),
                PatuiData::Known(PatuiDataInner::List(vec![
                    PatuiData::Known(PatuiDataInner::String("hello".to_string())),
                    PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
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
}
