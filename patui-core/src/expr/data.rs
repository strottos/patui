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
    /// Data is still pending but it cannot change. This is used when we know the data but
    /// something underneath this level is still unknown. Anything which has only Known things
    /// underneath it can (and should) be changed to known.
    ///
    /// The obvious example of this is a list of data that is being streamed in from another
    /// source where we got 4 elements but we know there are 5. We can't say for sure that this is
    /// known as there are definitely 5 elements but there is a missing 5th element (Milla Jovovich?)
    /// so it's not yet fully known. When the final element is added it becomes known, if it
    /// doesn't then we must error.
    PendingFixed(PatuiDataInner),
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
            PatuiData::Known(data) | PatuiData::PendingFixed(data) => data.is_known(),
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
            PatuiData::PendingFixed(_) => true,
            PatuiData::Pending(_) => true,
            PatuiData::Unknown => false,
        }
    }

    /// Return true if any of the data is unknown.
    pub fn is_unknown(&self) -> bool {
        match self {
            PatuiData::Known(patui_data_inner) => patui_data_inner.is_unknown(),
            PatuiData::PendingFixed(patui_data_inner) => patui_data_inner.is_unknown(),
            PatuiData::Pending(patui_data_inner) => patui_data_inner.is_unknown(),
            PatuiData::Unknown => true,
        }
    }

    /// Convert the data to a known state. Anything underneath the PatuiData element passed will
    /// also become `Known`.
    // TODO: how does this work with PendingFixed?
    pub fn to_known(self) -> Result<PatuiData, PatuiDataError> {
        let unknown = self.is_unknown();
        match self {
            PatuiData::PendingFixed(inner)
            | PatuiData::Pending(inner)
            | PatuiData::Known(inner) => {
                if unknown {
                    Ok(PatuiData::PendingFixed(inner.to_known_where_possible()))
                } else {
                    Ok(PatuiData::Known(inner.to_known()?))
                }
            }
            PatuiData::Unknown => Err(PatuiDataError::UnknownData),
        }
    }

    /// Convert the data to a known state. Anything underneath the PatuiData element passed will
    /// also become `Known`.
    // TODO: how does this work with PendingFixed?
    pub fn to_known_where_possible(self) -> PatuiData {
        let unknown = self.is_unknown();
        match self {
            PatuiData::PendingFixed(inner)
            | PatuiData::Pending(inner)
            | PatuiData::Known(inner) => {
                if unknown {
                    PatuiData::PendingFixed(inner.to_known_where_possible())
                } else {
                    PatuiData::Known(inner.to_known_where_possible())
                }
            }
            PatuiData::Unknown => PatuiData::Unknown,
        }
    }

    /// Get the inner data from the PatuiData
    ///
    /// Returns a boolean indicating if the data is known and a reference to the inner data.
    pub fn get_inner(&self) -> Result<(bool, &PatuiDataInner), PatuiDataError> {
        match self {
            PatuiData::Known(inner) => Ok((true, inner)),
            PatuiData::PendingFixed(inner) => Ok((inner.is_known(), inner)),
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
            PatuiData::PendingFixed(inner) => Ok((inner.is_known(), inner)),
            PatuiData::Pending(inner) => Ok((false, inner)),
            PatuiData::Unknown => Err(PatuiDataError::UnknownData),
        }
    }

    fn get_key_mut(&mut self, arg: &str) -> Result<(bool, &mut PatuiData), PatuiDataError> {
        let (known, inner) = self.get_inner_mut()?;
        match inner {
            PatuiDataInner::Map(hash_map) => match hash_map.get_mut(arg) {
                Some(data) => Ok((known, data)),
                None => Err(PatuiDataError::UnknownData),
            },
            _ => Err(PatuiDataError::WrongType("Map".to_string())),
        }
    }

    /// Add the step result to the results PatuiData as appropriate.
    pub fn add_step_result(&mut self, step_result: &PatuiStepResult) -> Result<(), PatuiDataError> {
        match &step_result.details {
            PatuiStepResultInner::StreamData(idx, patui_data) => {
                self.append_to_stream(&step_result.expr, *idx, patui_data)?;
            }
            PatuiStepResultInner::DoneStream(len) => {
                self.done_stream(&step_result.expr, *len)?;
            }
            _ => {
                return Err(PatuiDataError::BadStreamStepResult(
                    step_result.details.clone(),
                ))
            }
        }

        Ok(())
    }

    fn append_to_stream(
        &mut self,
        expr: &PatuiExpr,
        idx: usize,
        value: &PatuiData,
    ) -> Result<(), PatuiDataError> {
        let PatuiDataInner::List(ref mut vec) = self.create_data_stream(expr)? else {
            unreachable!();
        };

        tracing::trace!(
            "Adding data to stream at index {} with data length {}",
            idx,
            vec.len()
        );
        while vec.len() < idx {
            vec.push(PatuiData::Unknown);
        }
        match vec.len().cmp(&idx) {
            std::cmp::Ordering::Less => unreachable!(),
            std::cmp::Ordering::Equal => {
                vec.push(value.clone());
            }
            std::cmp::Ordering::Greater => {
                let _ = std::mem::replace(&mut vec[idx], value.clone());
                match expr.expr() {
                    Expr::Term(term_parts) => {
                        let mut parts_iter = term_parts.iter();
                        match parts_iter.next() {
                            Some(TermPart::Ident(f)) => {
                                if f != "steps" {
                                    return Err(PatuiDataError::UnknownData);
                                }
                            }
                            _ => return Err(PatuiDataError::UnknownData),
                        }
                        let mut current = self.get_key_mut("steps")?.1;

                        for part in parts_iter {
                            match part {
                                TermPart::Ident(ident) => {
                                    current = current.get_key_mut(ident)?.1;
                                }
                                _ => panic!("TODO: Code {:?}", part),
                            }
                        }

                        *current = current.clone().to_known()?;
                    }
                    _ => return Err(PatuiDataError::WrongType("Term".to_string())),
                }
            }
        }

        Ok(())
    }

    fn done_stream(&mut self, expr: &PatuiExpr, list_len: usize) -> Result<(), PatuiDataError> {
        self.create_data_stream(expr)?;

        match expr.expr() {
            Expr::Term(term_parts) => {
                let mut parts_iter = term_parts.iter();
                match parts_iter.next() {
                    Some(TermPart::Ident(f)) => {
                        if f != "steps" {
                            return Err(PatuiDataError::UnknownData);
                        }
                    }
                    _ => return Err(PatuiDataError::UnknownData),
                }
                let mut current = self.get_key_mut("steps")?.1;

                for part in parts_iter {
                    match part {
                        TermPart::Ident(ident) => {
                            current = current.get_key_mut(ident)?.1;
                        }
                        _ => panic!("TODO: Code {:?}", part),
                    }
                }

                {
                    let PatuiDataInner::List(ref mut vec) = current.get_inner_mut()?.1 else {
                        return Err(PatuiDataError::UnknownData);
                    };

                    while vec.len() < list_len {
                        vec.push(PatuiData::Unknown);
                    }
                }

                *current = current.clone().to_known()?;
            }
            _ => return Err(PatuiDataError::CantCreateData(expr.clone())),
        }

        Ok(())
    }

    fn create_data_stream(
        &mut self,
        expr: &PatuiExpr,
    ) -> Result<&mut PatuiDataInner, PatuiDataError> {
        let mut inner = self.get_inner_mut()?.1;

        match expr.expr() {
            Expr::Term(term_parts) => {
                for (idx, part) in term_parts.iter().enumerate() {
                    if let TermPart::Ident(ident) = part {
                        inner = match inner {
                            PatuiDataInner::Map(hash_map) => {
                                let entry =
                                    hash_map.entry(ident.to_string()).or_insert_with(|| {
                                        if idx == term_parts.len() - 1 {
                                            PatuiData::Pending(PatuiDataInner::List(vec![]))
                                        } else {
                                            PatuiData::Pending(PatuiDataInner::Map(HashMap::new()))
                                        }
                                    });
                                entry.get_inner_mut()?.1
                            }
                            _ => return Err(PatuiDataError::CantCreateData(expr.clone())),
                        };
                    }
                }
            }
            _ => return Err(PatuiDataError::CantCreateData(expr.clone())),
        }

        Ok(inner)
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
            PatuiData::Known(_) => todo!(),
            PatuiData::PendingFixed(_) => todo!(),
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
            PatuiDataInner::Map(map) => PatuiDataInner::Map(
                map.into_iter()
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

    fn to_known_where_possible(self) -> PatuiDataInner {
        match self {
            PatuiDataInner::List(vec) => PatuiDataInner::List(
                vec.into_iter()
                    .map(|data| data.to_known_where_possible())
                    .collect::<Vec<PatuiData>>(),
            ),
            PatuiDataInner::Map(map) => PatuiDataInner::Map(
                map.into_iter()
                    .map(|(key, data)| (key, data.to_known_where_possible()))
                    .collect::<HashMap<String, PatuiData>>(),
            ),
            PatuiDataInner::Set(vec) => PatuiDataInner::Set(
                vec.into_iter()
                    .map(|data| data.to_known_where_possible())
                    .collect::<Vec<PatuiData>>(),
            ),
            _ => self,
        }
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

    use super::{PatuiData, PatuiDataInner, PatuiDataListMethods};

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

        let data = PatuiData::PendingFixed(PatuiDataInner::Integer(42));
        assert_that!(data.is_known()).is_true();
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

        let data = PatuiDataInner::Map(
            vec![
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(42)),
                ),
                (
                    "key".to_string(),
                    PatuiData::PendingFixed(PatuiDataInner::Map(
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
    }

    #[traced_test]
    #[test]
    fn patui_data_to_known_for_pending_fixed() {
        let data = PatuiData::Pending(PatuiDataInner::List(vec![
            PatuiData::Pending(PatuiDataInner::Integer(42)),
            PatuiData::Unknown,
            PatuiData::Pending(PatuiDataInner::Integer(42)),
            PatuiData::Unknown,
        ]));
        let data = data.to_known();

        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiData::PendingFixed(PatuiDataInner::List(
            vec![
                PatuiData::Known(PatuiDataInner::Integer(42)),
                PatuiData::Unknown,
                PatuiData::Known(PatuiDataInner::Integer(42)),
                PatuiData::Unknown,
            ],
        )));

        let data = PatuiData::PendingFixed(PatuiDataInner::List(vec![
            PatuiData::Pending(PatuiDataInner::Integer(42)),
            PatuiData::Pending(PatuiDataInner::Integer(42)),
            PatuiData::Pending(PatuiDataInner::Integer(42)),
            PatuiData::Pending(PatuiDataInner::Integer(42)),
        ]));
        let data = data.to_known();

        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiData::Known(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Integer(42)),
            PatuiData::Known(PatuiDataInner::Integer(42)),
            PatuiData::Known(PatuiDataInner::Integer(42)),
            PatuiData::Known(PatuiDataInner::Integer(42)),
        ])));

        let data = PatuiData::Pending(PatuiDataInner::Map(HashMap::from([
            (
                "key1".to_string(),
                PatuiData::Pending(PatuiDataInner::Integer(42)),
            ),
            ("key2".to_string(), PatuiData::Unknown),
        ])));
        let data = data.to_known();

        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiData::PendingFixed(PatuiDataInner::Map(
            HashMap::from([
                (
                    "key1".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(42)),
                ),
                ("key2".to_string(), PatuiData::Unknown),
            ]),
        )));
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
    fn append_stream_step_result_to_data() {
        let mut results = PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
            "steps".to_string(),
            PatuiData::Pending(PatuiDataInner::Map(HashMap::new())),
        )])));

        let ret = results.add_step_result(&PatuiStepResult::new_stream_item(
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

        let ret = results.add_step_result(&PatuiStepResult::new_stream_item(
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

    #[traced_test]
    #[test]
    fn finish_stream_step_result_when_correct_len() {
        let mut results = PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
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
                            PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                        ])),
                    )]))),
                )]))),
            )]))),
        )])));

        let ret = results.add_step_result(&PatuiStepResult::done_stream(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            3,
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
                            PatuiData::Known(PatuiDataInner::List(vec![
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                                PatuiData::Known(PatuiDataInner::String("test2".to_string())),
                                PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));
    }

    #[traced_test]
    #[test]
    fn finish_stream_step_result_with_unknown_data() {
        let mut results = PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
            "steps".to_string(),
            PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                "step_test".to_string(),
                PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                    "func_test".to_string(),
                    PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                        "out_test".to_string(),
                        PatuiData::Pending(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                            PatuiData::Unknown,
                            PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                        ])),
                    )]))),
                )]))),
            )]))),
        )])));

        let ret = results.add_step_result(&PatuiStepResult::done_stream(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            3,
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
                            PatuiData::PendingFixed(PatuiDataInner::List(vec![
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                                PatuiData::Unknown,
                                PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));

        let mut results = PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
            "steps".to_string(),
            PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                "step_test".to_string(),
                PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                    "func_test".to_string(),
                    PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
                        "out_test".to_string(),
                        PatuiData::Pending(PatuiDataInner::List(vec![
                            PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                            PatuiData::Unknown,
                            PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                        ])),
                    )]))),
                )]))),
            )]))),
        )])));

        let ret = results.add_step_result(&PatuiStepResult::done_stream(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            4,
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
                            PatuiData::PendingFixed(PatuiDataInner::List(vec![
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                                PatuiData::Unknown,
                                PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                                PatuiData::Unknown,
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));
    }

    #[traced_test]
    #[test]
    fn append_stream_step_result_to_data_works_out_of_order() {
        let mut results = PatuiData::Pending(PatuiDataInner::Map(HashMap::from([(
            "steps".to_string(),
            PatuiData::Pending(PatuiDataInner::Map(HashMap::new())),
        )])));

        let ret = results.add_step_result(&PatuiStepResult::new_stream_item(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            1,
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
                            PatuiData::Pending(PatuiDataInner::List(vec![
                                PatuiData::Unknown,
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));

        let ret = results.add_step_result(&PatuiStepResult::done_stream(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            4,
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
                            PatuiData::PendingFixed(PatuiDataInner::List(vec![
                                PatuiData::Unknown,
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                                PatuiData::Unknown,
                                PatuiData::Unknown,
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));

        let ret = results.add_step_result(&PatuiStepResult::new_stream_item(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            3,
            PatuiData::Known(PatuiDataInner::String("test3".to_string())),
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
                            PatuiData::PendingFixed(PatuiDataInner::List(vec![
                                PatuiData::Unknown,
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                                PatuiData::Unknown,
                                PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));

        let ret = results.add_step_result(&PatuiStepResult::new_stream_item(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            2,
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
                            PatuiData::PendingFixed(PatuiDataInner::List(vec![
                                PatuiData::Unknown,
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                                PatuiData::Known(PatuiDataInner::String("test2".to_string())),
                                PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));

        let ret = results.add_step_result(&PatuiStepResult::new_stream_item(
            "steps.step_test.func_test.out_test".try_into().unwrap(),
            PatuiStepResultStatus::Success,
            0,
            PatuiData::Known(PatuiDataInner::String("test0".to_string())),
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
                            PatuiData::Known(PatuiDataInner::List(vec![
                                PatuiData::Known(PatuiDataInner::String("test0".to_string())),
                                PatuiData::Known(PatuiDataInner::String("test1".to_string())),
                                PatuiData::Known(PatuiDataInner::String("test2".to_string())),
                                PatuiData::Known(PatuiDataInner::String("test3".to_string())),
                            ])),
                        )]))),
                    )]))),
                )]))),
            )],
        ))));
    }

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
