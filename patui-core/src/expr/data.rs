use std::collections::HashMap;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use thiserror::Error;

use crate::{PatuiStepResult, PatuiStepResultInner};

use super::{
    ast::{Expr, TermPart},
    EvalError, PatuiExpr,
};

#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum PatuiDataError {
    #[error("Data is unknown")]
    UnknownData,
    #[error("Type is unknown")]
    UnknownType,
    #[error("Wrong Type, expected '{0}'")]
    WrongType(String),
    #[error("Bad arguments for function {0}")]
    BadArgs(String),
    #[error("Bad stream step result: {0:?}")]
    BadStreamStepResult(PatuiStepResultInner),
    #[error("Bad data merge: {0}")]
    BadMerge(String),
    #[error("Location error: {0} - {1}")]
    LocationError(PatuiExpr, Box<EvalError>),
    #[error("Can't create data from expression: {0}")]
    CantCreateData(PatuiExpr),
}

/// Data type for the evaluation of expressions. Any evaluation of an expression must evaluate to
/// one of these types.
///
/// There are three possible cases here:
/// - Known: The data is known and can be used in the evaluation of expressions. If this is the
///   case the element in question will never change and the PatuiDataInner structure can be
///   trusted to always evaluate to the same result. Not that there may still be things underneath
///   a known structure that are pending or unknown.
/// - Pending: The data is pending and may change in the future. This is used when we don't know
///   for sure about the data yet, for example results streaming are still being streamed from
///   another step.
/// - Unknown: The data is unknown and we don't know anything about it. This is used when we can't
///   rely on the data yet. If the data is finalised then this will evaluate to an error but there
///   are times when data is pending and we use unknown in this case.
///
/// In most cases it is expected that data is going to be known, the main countercase is as
/// follows. If we have a list of data that we are streaming in from another source and we ask for
/// the length of that list, we can't know the length of the list until we have all the data so we
/// mark this as 'Pending'. If we ask for the 5th element of that list but we only have 3 so far
/// and it's still pending, then requesting that data could mean it's just not in yet, or it could
/// mean that it's never going to be there in which case it's an error. We can't say for sure it's
/// an error though so we mark it as unknown.
///
/// On the other hand, if I ask for the 5th element of a list that I know has only 3 elements this
/// is an error and should be marked as such.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PatuiData {
    /// Data is known and can be used definitely and for all the in the evaluation of expressions.
    Known(PatuiDataInner),
    /// Data is pending and may change in the future. This is used when we don't know for sure
    /// about the data in question yet and have to wait for it to be finalised to evaluate it.
    Pending(PatuiDataInner),
    /// Data is unknown and we don't know anything about it yet. This is used when we have a
    /// pending data structure that we evaluated that we can't rely on yet.
    Unknown,
}

/// Inner data type for the evaluation of expressions. This is the actual data that is being used
/// by Patui.
///
/// This is very similar to a JSON/YAML/etc type of structure elements though with some slight
/// variation as interesting to Patui such as the Bytes type.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Display)]
pub enum PatuiDataInner {
    /// Null data type. This is used when we have no data.
    Null,
    /// Boolean data type. This is used when we have a boolean value.
    Bool(bool),
    /// Bytes data type. This is used when we have an array of byte values.
    Bytes(Bytes),
    /// String data type. This is used when we have a UTF-8 encoded string value.
    String(String),
    /// Integer data type. This is used when we have an integer value. Only signed 64-bit integers
    /// are supported at present.
    Integer(i64),
    /// Decimal data type. This is used when we have a decimal value. Only 64-bit floating point
    /// numbers are supported at present.
    Decimal(f64),
    /// List data type. This is an ordered list of `PatuiData` elements.
    List(Vec<PatuiData>),
    /// Map data type. This is an unordered map of `PatuiData` elements with keys as strings.
    Map(HashMap<String, PatuiData>),
    /// Set data type. This is an unordered set of `PatuiData` elements.
    Set(Vec<PatuiData>),
}

impl PatuiData {
    /// Return true if the data is all known.
    pub fn is_known(&self) -> bool {
        match self {
            PatuiData::Known(data) => data.is_known(),
            _ => false,
        }
    }

    /// Return true if any of the data is pending but nothing is unknown.
    pub fn is_pending(&self) -> bool {
        if self.is_unknown() {
            return false;
        }
        match self {
            PatuiData::Known(patui_data_inner) => patui_data_inner.is_pending(),
            PatuiData::Pending(_) => true,
            PatuiData::Unknown => false,
        }
    }

    /// Return true if any of the data is unknown.
    pub fn is_unknown(&self) -> bool {
        match self {
            PatuiData::Known(patui_data_inner) => patui_data_inner.is_unknown(),
            PatuiData::Pending(patui_data_inner) => patui_data_inner.is_unknown(),
            PatuiData::Unknown => true,
        }
    }

    /// Convert the data to a known state. Anything underneath the PatuiData element passed will
    /// also become `Known`.
    pub fn to_known(self) -> Result<PatuiData, PatuiDataError> {
        let inner = match self {
            PatuiData::Pending(inner) => inner,
            PatuiData::Known(inner) => inner,
            PatuiData::Unknown => return Err(PatuiDataError::UnknownData),
        };

        Ok(PatuiData::Known(inner.to_known()?))
    }

    /// Get the inner data from the PatuiData
    ///
    /// Returns a boolean indicating if the data is known and a reference to the inner data.
    pub fn get_inner(&self) -> Result<(bool, &PatuiDataInner), PatuiDataError> {
        match self {
            PatuiData::Known(inner) => Ok((true, inner)),
            PatuiData::Pending(inner) => Ok((false, inner)),
            PatuiData::Unknown => Err(PatuiDataError::UnknownData),
        }
    }

    /// Get the inner data from the PatuiData
    ///
    /// Returns a boolean indicating if the data is known and a mutable reference to the inner
    /// data.
    pub fn get_inner_mut(&mut self) -> Result<(bool, &mut PatuiDataInner), PatuiDataError> {
        match self {
            PatuiData::Known(inner) => Ok((true, inner)),
            PatuiData::Pending(inner) => Ok((false, inner)),
            PatuiData::Unknown => Err(PatuiDataError::UnknownData),
        }
    }

    /// Given a list of keys it recursively search through maps to find the keys until it finds a
    /// list, it will then append `value` to that list. If it finds missing keys it will add them,
    /// if it finds something existing with the wrong type it will error out.
    ///
    /// This is the one of the most common operation we perform on Patui results.
    pub fn append_to_list(
        &mut self,
        keys: Vec<String>,
        value: PatuiData,
    ) -> Result<(), PatuiDataError> {
        let (_, mut inner) = self.get_inner_mut()?;
        for (i, key) in keys.iter().enumerate() {
            inner = match inner {
                PatuiDataInner::Map(hash_map) => {
                    let entry = hash_map.entry(key.to_string()).or_insert_with(|| {
                        if i == keys.len() - 1 {
                            PatuiData::Pending(PatuiDataInner::List(vec![]))
                        } else {
                            PatuiData::Pending(PatuiDataInner::Map(HashMap::new()))
                        }
                    });
                    entry.get_inner_mut()?.1
                }
                _ => return Err(PatuiDataError::WrongType("Map".to_string())),
            };
        }

        tracing::trace!("Found data: {:?}", inner);

        match inner {
            PatuiDataInner::List(vec) => {
                vec.push(value);
            }
            _ => return Err(PatuiDataError::WrongType("List".to_string())),
        }

        Ok(())
    }

    /// Add the data to the stream at the specified index. This is used when we have
    pub fn add_step_result_to_stream(
        &mut self,
        step_result: &PatuiStepResult,
    ) -> Result<(), PatuiDataError> {
        let data = self.create_data_stream(&step_result.location)?;

        tracing::trace!("Data {:?}", data);
        let PatuiData::Pending(PatuiDataInner::List(ref mut data)) = data else {
            todo!();
        };

        match &step_result.details {
            crate::PatuiStepResultInner::StreamData(idx, patui_data) => {
                if data.len() == *idx {
                    data.push(patui_data.clone());
                } else {
                    todo!();
                }
            }
            _ => {
                return Err(PatuiDataError::BadStreamStepResult(
                    step_result.details.clone(),
                ))
            }
        }

        Ok(())
    }

    fn create_data_stream(&mut self, expr: &PatuiExpr) -> Result<&mut PatuiData, PatuiDataError> {
        let mut current = self;

        match expr.expr() {
            Expr::Term(term_parts) => {
                for (idx, part) in term_parts.into_iter().enumerate() {
                    if let TermPart::Ident(ident) = part {
                        match current {
                            PatuiData::Known(patui_data_inner)
                            | PatuiData::Pending(patui_data_inner) => match patui_data_inner {
                                PatuiDataInner::Map(hash_map) => {
                                    let entry =
                                        hash_map.entry(ident.to_string()).or_insert_with(|| {
                                            if idx == term_parts.len() - 1 {
                                                PatuiData::Pending(PatuiDataInner::List(vec![]))
                                            } else {
                                                PatuiData::Pending(PatuiDataInner::Map(
                                                    HashMap::new(),
                                                ))
                                            }
                                        });
                                    current = entry;
                                }
                                _ => return Err(PatuiDataError::CantCreateData(expr.clone())),
                            },
                            PatuiData::Unknown => {
                                return Err(PatuiDataError::CantCreateData(expr.clone()))
                            }
                        }
                    }
                }
            }
            _ => return Err(PatuiDataError::CantCreateData(expr.clone())),
        }

        Ok(current)
    }

    /// Merge the data from `other` into `self`. This is used when we have something to append to a
    /// `results` data structure and we want to add the new data to the existing data.
    ///
    /// The rules for merging are as follows:
    /// - If this data is known and the other data is known then they must be equal, if they are
    ///   equal then we keep the data as known, if they are not equal then we error out.
    /// - If this data is known and the other data is either pending or unknown then we must throw
    ///   an error.
    /// - If this data is pending and the other data is known or pending then we merge the new data
    ///   into the existing data as known.
    /// - If this data is pending and the other data is unknown then we throw an error. TODO: Any
    ///   conceivable use case for this to not error?
    /// - If this data is unknown then we merge the new data into the existing data trivially.
    ///
    /// If we find any new keys in maps we create them.
    pub(crate) fn merge(&mut self, other: &PatuiData) -> Result<(), PatuiDataError> {
        match self {
            PatuiData::Known(patui_data_inner) => todo!(),
            PatuiData::Pending(_) => {
                if other.is_unknown() {
                    return Err(PatuiDataError::BadMerge("TODO".to_string()));
                }
                *self = other.clone();
            }
            PatuiData::Unknown => {
                if other.is_unknown() {
                    return Ok(());
                };
                *self = other.clone();
            }
        }

        Ok(())
    }
}

impl PatuiDataInner {
    /// Get the key specified from the data as a map. If the data is not a map we error out.
    pub fn get_map_key(&self, key: &str) -> Result<&PatuiData, PatuiDataError> {
        match self {
            PatuiDataInner::Map(hash_map) => match hash_map.get(key) {
                Some(data) => Ok(data),
                None => Err(PatuiDataError::UnknownData),
            },
            _ => Err(PatuiDataError::WrongType("Map".to_string())),
        }
    }

    /// Return true if the data is all known.
    fn is_known(&self) -> bool {
        match self {
            PatuiDataInner::List(vec) => vec.iter().all(|data| data.is_known()),
            PatuiDataInner::Map(hash_map) => hash_map.values().all(|data| data.is_known()),
            PatuiDataInner::Set(vec) => vec.iter().all(|data| data.is_known()),
            _ => true,
        }
    }

    /// Return true if any of the data is pending and none of it is unknown.
    fn is_pending(&self) -> bool {
        match self {
            PatuiDataInner::List(vec) => vec.iter().any(|data| data.is_pending()),
            PatuiDataInner::Map(hash_map) => hash_map.values().any(|data| data.is_pending()),
            PatuiDataInner::Set(vec) => vec.iter().any(|data| data.is_pending()),
            _ => false,
        }
    }

    /// Return true if any of the data is unknown.
    fn is_unknown(&self) -> bool {
        match self {
            PatuiDataInner::List(vec) => vec.iter().any(|data| data.is_unknown()),
            PatuiDataInner::Map(hash_map) => hash_map.values().any(|data| data.is_unknown()),
            PatuiDataInner::Set(vec) => vec.iter().any(|data| data.is_unknown()),
            _ => false,
        }
    }

    fn to_known(self) -> Result<PatuiDataInner, PatuiDataError> {
        Ok(match self {
            PatuiDataInner::List(vec) => PatuiDataInner::List(
                vec.into_iter()
                    .map(|data| data.to_known())
                    .collect::<Result<Vec<PatuiData>, PatuiDataError>>()?,
            ),
            PatuiDataInner::Map(hash_map) => PatuiDataInner::Map(
                hash_map
                    .into_iter()
                    .map(|(key, data)| Ok((key, data.to_known()?)))
                    .collect::<Result<HashMap<String, PatuiData>, PatuiDataError>>()?,
            ),
            PatuiDataInner::Set(vec) => PatuiDataInner::Set(
                vec.into_iter()
                    .map(|data| data.to_known())
                    .collect::<Result<Vec<PatuiData>, PatuiDataError>>()?,
            ),
            _ => self,
        })
    }
}

// Method calls for each data type

/// Methods for the list data type
pub(crate) struct PatuiDataListMethods<'a> {
    list: &'a [PatuiData],
    known: bool,
}

impl<'a> PatuiDataListMethods<'a> {
    /// Create a new instance of the list data type
    pub(crate) fn new(list: &'a [PatuiData], known: bool) -> Self {
        PatuiDataListMethods { list, known }
    }

    /// Get the current length of the list.
    ///
    /// If we're known we return a known number. If not we reutrn a pending number.
    pub(crate) fn len(self, args: &[Expr]) -> Result<PatuiData, PatuiDataError> {
        if !args.is_empty() {
            return Err(PatuiDataError::BadArgs("len".to_string()));
        }

        if self.known {
            Ok(PatuiData::Known(PatuiDataInner::Integer(
                self.list.len() as i64, // TODO: Can we overflow on any platforms, what if we do?
            )))
        } else {
            Ok(PatuiData::Pending(PatuiDataInner::Integer(
                self.list.len() as i64
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use assertor::*;
    use tracing_test::traced_test;

    use crate::runner::{PatuiStepResult, PatuiStepResultStatus};

    use super::{PatuiData, PatuiDataError, PatuiDataInner, PatuiDataListMethods};

    #[traced_test]
    #[test]
    fn patui_data() {
        let data = PatuiData::Pending(PatuiDataInner::Integer(42));
        assert_that!(data.is_known()).is_false();
        assert_that!(data.is_pending()).is_true();
        assert_that!(data.is_unknown()).is_false();
        let data = data.to_known();
        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiData::Known(PatuiDataInner::Integer(42)));

        let data = PatuiData::Known(PatuiDataInner::Integer(42));
        assert_that!(data.is_known()).is_true();
        assert_that!(data.is_pending()).is_false();
        assert_that!(data.is_unknown()).is_false();
        let data = data.to_known();
        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiData::Known(PatuiDataInner::Integer(42)));

        let data = PatuiData::Unknown;
        assert_that!(data.is_known()).is_false();
        assert_that!(data.is_pending()).is_false();
        assert_that!(data.is_unknown()).is_true();
        let data = data.to_known();
        assert_that!(data).is_err();
    }

    #[traced_test]
    #[test]
    fn patui_data_inner() {
        let data = PatuiDataInner::List(vec![
            PatuiData::Pending(PatuiDataInner::Integer(42)),
            PatuiData::Pending(PatuiDataInner::Integer(42)),
        ]);
        let data = data.to_known();
        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Integer(42)),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));

        let data = PatuiDataInner::Map(
            vec![
                (
                    "key".to_string(),
                    PatuiData::Pending(PatuiDataInner::Integer(42)),
                ),
                (
                    "key".to_string(),
                    PatuiData::Pending(PatuiDataInner::Integer(42)),
                ),
            ]
            .into_iter()
            .collect(),
        );
        let data = data.to_known();
        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiDataInner::Map(
            vec![
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(42)),
                ),
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(42)),
                ),
            ]
            .into_iter()
            .collect(),
        ));

        let data = PatuiDataInner::Map(
            vec![
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(42)),
                ),
                (
                    "key".to_string(),
                    PatuiData::Pending(PatuiDataInner::Map(
                        vec![
                            (
                                "key".to_string(),
                                PatuiData::Known(PatuiDataInner::Integer(42)),
                            ),
                            (
                                "key".to_string(),
                                PatuiData::Pending(PatuiDataInner::Integer(42)),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    )),
                ),
            ]
            .into_iter()
            .collect(),
        );
        let data = data.to_known();
        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiDataInner::Map(
            vec![
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(42)),
                ),
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Map(
                        vec![
                            (
                                "key".to_string(),
                                PatuiData::Known(PatuiDataInner::Integer(42)),
                            ),
                            (
                                "key".to_string(),
                                PatuiData::Known(PatuiDataInner::Integer(42)),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    )),
                ),
            ]
            .into_iter()
            .collect(),
        ));
    }

    #[traced_test]
    #[test]
    fn patui_data_to_known_with_unknown() {
        let data = PatuiData::Unknown;
        let data = data.to_known();
        assert_that!(data).is_err();

        let data = PatuiData::Pending(PatuiDataInner::Map(HashMap::from([
            (
                "key1".to_string(),
                PatuiData::Pending(PatuiDataInner::Integer(42)),
            ),
            ("key2".to_string(), PatuiData::Unknown),
        ])));
        let data = data.to_known();
        assert_that!(data).is_err();
    }

    #[traced_test]
    #[test]
    fn patui_list_length() {
        let inner_list = vec![
            PatuiData::Known(PatuiDataInner::Integer(42)),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ];

        let patui_list_data_functions = PatuiDataListMethods::new(&inner_list, true);
        let len = patui_list_data_functions.len(&[]);

        assert_that!(len).is_ok();
        assert_that!(len.unwrap()).is_equal_to(PatuiData::Known(PatuiDataInner::Integer(2)));
    }

    #[traced_test]
    #[test]
    fn data_is_known() {
        let data = PatuiData::Known(PatuiDataInner::Integer(42));
        assert_that!(data.is_known()).is_true();

        let data = PatuiData::Pending(PatuiDataInner::Integer(42));
        assert_that!(data.is_known()).is_false();

        let data = PatuiData::Unknown;
        assert_that!(data.is_known()).is_false();

        let data = PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "key".to_string(),
                PatuiData::Known(PatuiDataInner::Integer(42)),
            )]))),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));
        assert_that!(data.is_known()).is_true();

        let data = PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "key".to_string(),
                PatuiData::Pending(PatuiDataInner::Integer(42)),
            )]))),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));
        assert_that!(data.is_known()).is_false();
    }

    #[traced_test]
    #[test]
    fn data_is_unknown() {
        let data = PatuiData::Known(PatuiDataInner::Integer(42));
        assert_that!(data.is_unknown()).is_false();

        let data = PatuiData::Pending(PatuiDataInner::Integer(42));
        assert_that!(data.is_unknown()).is_false();

        let data = PatuiData::Unknown;
        assert_that!(data.is_unknown()).is_true();

        let data = PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "key".to_string(),
                PatuiData::Known(PatuiDataInner::Integer(42)),
            )]))),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));
        assert_that!(data.is_unknown()).is_false();

        let data = PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "key".to_string(),
                PatuiData::Pending(PatuiDataInner::Integer(42)),
            )]))),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));
        assert_that!(data.is_unknown()).is_false();

        let data = PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "key".to_string(),
                PatuiData::Unknown,
            )]))),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));
        assert_that!(data.is_unknown()).is_true();
    }

    #[traced_test]
    #[test]
    fn data_is_pending() {
        let data = PatuiData::Known(PatuiDataInner::Integer(42));
        assert_that!(data.is_pending()).is_false();

        let data = PatuiData::Pending(PatuiDataInner::Integer(42));
        assert_that!(data.is_pending()).is_true();

        let data = PatuiData::Unknown;
        assert_that!(data.is_pending()).is_false();

        let data = PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "key".to_string(),
                PatuiData::Known(PatuiDataInner::Integer(42)),
            )]))),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));
        assert_that!(data.is_pending()).is_false();

        let data = PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "key".to_string(),
                PatuiData::Pending(PatuiDataInner::Integer(42)),
            )]))),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));
        assert_that!(data.is_pending()).is_true();

        let data = PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                "key".to_string(),
                PatuiData::Unknown,
            )]))),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ]));
        assert_that!(data.is_pending()).is_false();
    }

    #[traced_test]
    #[test]
    fn add_to_list() {
        let mut data = PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
            "key1".to_string(),
            PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "key2".to_string(),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "key3".to_string(),
                    PatuiData::Known(PatuiDataInner::List(vec![
                        PatuiData::Known(PatuiDataInner::Integer(1)),
                        PatuiData::Known(PatuiDataInner::Integer(2)),
                    ])),
                )]))),
            )]))),
        )])));

        assert_that!(data.append_to_list(
            vec!["key1".to_string(), "key2".to_string(), "key3".to_string()],
            PatuiData::Known(PatuiDataInner::Integer(3)),
        ))
        .is_ok();
        assert_that!(data.append_to_list(
            vec!["not".to_string(), "exists".to_string()],
            PatuiData::Known(PatuiDataInner::Integer(3)),
        ))
        .is_ok();

        let ret = data.get_inner();
        assert_that!(ret).is_ok();
        let (known, root_inner) = ret.unwrap();
        assert_that!(known).is_true();

        let ret = root_inner.get_map_key("key1");
        assert_that!(ret).is_ok();
        let data = ret.unwrap();
        let ret = data.get_inner();
        assert_that!(ret).is_ok();
        let (known, inner) = ret.unwrap();
        assert_that!(known).is_true();

        let ret = inner.get_map_key("key2");
        assert_that!(ret).is_ok();
        let data = ret.unwrap();
        let ret = data.get_inner();
        assert_that!(ret).is_ok();
        let (known, inner) = ret.unwrap();
        assert_that!(known).is_true();

        let ret = inner.get_map_key("key3");
        assert_that!(ret).is_ok();
        let data = ret.unwrap();
        let ret = data.get_inner();
        assert_that!(ret).is_ok();
        let (known, inner) = ret.unwrap();
        assert_that!(known).is_true();

        assert_that!(inner).is_equal_to(&PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Integer(1)),
            PatuiData::Known(PatuiDataInner::Integer(2)),
            PatuiData::Known(PatuiDataInner::Integer(3)),
        ]));

        let ret = root_inner.get_map_key("not");
        assert_that!(ret).is_ok();
        let data = ret.unwrap();
        let ret = data.get_inner();
        assert_that!(ret).is_ok();
        let (known, inner) = ret.unwrap();
        assert_that!(known).is_false();

        let ret = inner.get_map_key("exists");
        assert_that!(ret).is_ok();
        let data = ret.unwrap();
        let ret = data.get_inner();
        assert_that!(ret).is_ok();
        let (known, inner) = ret.unwrap();
        assert_that!(known).is_false();

        assert_that!(inner).is_equal_to(&PatuiDataInner::List(vec![PatuiData::Known(
            PatuiDataInner::Integer(3),
        )]));
    }

    #[traced_test]
    #[test]
    fn merge_to_unknown() {
        let mut data = PatuiData::Unknown;

        let other = PatuiData::Unknown;
        assert_that!(data.merge(&other)).is_ok();

        assert_that!(data).is_equal_to(&PatuiData::Unknown);

        let other = PatuiData::Pending(PatuiDataInner::Integer(42));
        assert_that!(data.merge(&other)).is_ok();

        let ret = data.get_inner();
        assert_that!(ret).is_ok();
        let (known, inner) = ret.unwrap();
        assert_that!(known).is_false();
        assert_that!(inner).is_equal_to(&PatuiDataInner::Integer(42));

        let mut data = PatuiData::Unknown;

        let other = PatuiData::Known(PatuiDataInner::Integer(42));
        assert_that!(data.merge(&other)).is_ok();

        let ret = data.get_inner();
        assert_that!(ret).is_ok();
        let (known, inner) = ret.unwrap();
        assert_that!(known).is_true();
        assert_that!(inner).is_equal_to(&PatuiDataInner::Integer(42));
    }

    #[traced_test]
    #[test]
    fn merge_to_pending() {
        let mut data = PatuiData::Pending(PatuiDataInner::Integer(42));

        let other = PatuiData::Unknown;
        let ret = data.merge(&other);

        assert_that!(ret).is_err();
        let error = ret.unwrap_err();
        assert_that!(error).is_equal_to(PatuiDataError::BadMerge("TODO".to_string()));

        let other = PatuiData::Pending(PatuiDataInner::Integer(43));
        let ret = data.merge(&other);

        assert_that!(ret).is_ok();
        let inner = data.get_inner();
        assert_that!(inner).is_ok();
        let (known, inner) = inner.unwrap();
        assert_that!(known).is_false();
        assert_that!(inner).is_equal_to(&PatuiDataInner::Integer(43));
    }

    // #[traced_test]
    // #[test]
    // fn merge_known_to_known_errors() {
    //     let mut data = PatuiData::Known(PatuiDataInner::List(vec![
    //         PatuiData::Known(PatuiDataInner::Integer(1)),
    //         PatuiData::Known(PatuiDataInner::Integer(2)),
    //     ]));

    //     let other = PatuiData::Known(PatuiDataInner::List(vec![
    //         PatuiData::Known(PatuiDataInner::Integer(1)),
    //         PatuiData::Known(PatuiDataInner::Integer(2)),
    //         PatuiData::Known(PatuiDataInner::Integer(3)),
    //     ]));
    //     assert_that!(data.merge(&other)).is_ok();

    //     let ret = data.get_inner();
    //     assert_that!(ret).is_err();
    //     let error = ret.unwrap_err();
    //     assert_that!(error).is_equal_to(PatuiDataError::BadMerge("TODO".to_string()));
    // }

    #[traced_test]
    #[test]
    fn append_stream_step_result_to_data() {
        let mut results = PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
            "steps".to_string(),
            PatuiData::Pending(PatuiDataInner::Map(HashMap::new())),
        )])));

        let ret = results.add_step_result_to_stream(&PatuiStepResult::new_stream_item(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            0,
            PatuiData::Known(PatuiDataInner::String("test1".to_string())),
        ));

        assert_that!(ret).is_ok();
        assert_that!(results).is_equal_to(&PatuiData::Pending(PatuiDataInner::Map(HashMap::from(
            [(
                "steps".to_string(),
                PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                    "step_test".to_string(),
                    PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                        "func_test".to_string(),
                        PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                            "out_test".to_string(),
                            PatuiData::Pending(PatuiDataInner::List(vec![PatuiData::Known(
                                PatuiDataInner::String("test1".to_string()),
                            )])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));

        let ret = results.add_step_result_to_stream(&PatuiStepResult::new_stream_item(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            1,
            PatuiData::Known(PatuiDataInner::String("test2".to_string())),
        ));

        assert_that!(ret).is_ok();
        assert_that!(results).is_equal_to(&PatuiData::Pending(PatuiDataInner::Map(HashMap::from(
            [(
                "steps".to_string(),
                PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                    "step_test".to_string(),
                    PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                        "func_test".to_string(),
                        PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                            "out_test".to_string(),
                            PatuiData::Pending(PatuiDataInner::List(vec![
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                                PatuiData::Known(PatuiDataInner::String("test2".to_string())),
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));
    }

    // #[traced_test]
    // #[test]
    // fn append_stream_step_result_to_data_works_out_of_order() {
    //     todo!()
    // }

    // #[traced_test]
    // #[test]
    // fn bad_step_result_expr_errors() {
    //     todo!()
    // }

    // #[traced_test]
    // #[test]
    // fn step_result_expr_append_to_stream_when_exists_non_stream_errors(){
    //     todo!()
    // }

    // #[traced_test]
    // #[test]
    // fn step_result_expr_append_to_stream_with_non_stream_item_errors(){
    //     todo!()
    // }
}
