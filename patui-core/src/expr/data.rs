use std::collections::HashMap;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use thiserror::Error;

use crate::ptplugin::PatuiDataEncoding;

use super::ast::Expr;

#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum PatuiDataError {
    #[error("Data is unknown")]
    UnknownData,
    #[error("Type is unknown")]
    UnknownType,
    #[error("Bad arguments for function {0}")]
    BadArgs(String),
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

    /// Convert the data to a known state.
    pub fn to_known(self) -> Result<PatuiData, PatuiDataError> {
        let inner = match self {
            PatuiData::Pending(inner) => inner,
            PatuiData::Known(inner) => inner,
            PatuiData::Unknown => return Err(PatuiDataError::UnknownData),
        };

        Ok(PatuiData::Known(inner.to_known()?))
    }
}

impl PatuiDataInner {
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

impl TryFrom<PatuiData> for PatuiDataEncoding {
    type Error = rmp_serde::encode::Error;

    fn try_from(value: PatuiData) -> Result<Self, Self::Error> {
        Ok(PatuiDataEncoding {
            bytes: rmp_serde::to_vec(&value)?,
        })
    }
}

impl TryFrom<PatuiDataEncoding> for PatuiData {
    type Error = rmp_serde::decode::Error;

    fn try_from(data: PatuiDataEncoding) -> Result<Self, Self::Error> {
        rmp_serde::from_read(data.bytes.as_slice())
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
    use assertor::*;

    use super::*;

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
}
