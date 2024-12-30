use std::collections::HashMap;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use thiserror::Error;

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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PatuiData {
    Pending(PatuiDataInner),
    Known(PatuiDataInner),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Display)]
pub enum PatuiDataInner {
    Null,
    Bool(bool),
    Bytes(Bytes),
    String(String),
    Integer(rug::Integer),
    Decimal(rug::Float),
    List(Vec<PatuiData>),
    Map(HashMap<String, PatuiData>),
    Set(Vec<PatuiData>),
}

impl PatuiData {
    pub fn is_known(&self) -> bool {
        matches!(self, PatuiData::Known(_))
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, PatuiData::Pending(_))
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, PatuiData::Unknown)
    }

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
    pub fn to_known(self) -> Result<PatuiDataInner, PatuiDataError> {
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
                rug::Integer::from(self.list.len()),
            )))
        } else {
            Ok(PatuiData::Pending(PatuiDataInner::Integer(
                rug::Integer::from(self.list.len()),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    #[test]
    fn test_patui_data() {
        let data = PatuiData::Pending(PatuiDataInner::Integer(rug::Integer::from(42)));
        assert_that!(data.is_known()).is_false();
        assert_that!(data.is_pending()).is_true();
        assert_that!(data.is_unknown()).is_false();
        let data = data.to_known();
        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiData::Known(PatuiDataInner::Integer(
            rug::Integer::from(42),
        )));

        let data = PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42)));
        assert_that!(data.is_known()).is_true();
        assert_that!(data.is_pending()).is_false();
        assert_that!(data.is_unknown()).is_false();
        let data = data.to_known();
        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiData::Known(PatuiDataInner::Integer(
            rug::Integer::from(42),
        )));

        let data = PatuiData::Unknown;
        assert_that!(data.is_known()).is_false();
        assert_that!(data.is_pending()).is_false();
        assert_that!(data.is_unknown()).is_true();
        let data = data.to_known();
        assert_that!(data).is_err();
    }

    #[test]
    fn test_patui_data_inner() {
        let data = PatuiDataInner::List(vec![
            PatuiData::Pending(PatuiDataInner::Integer(rug::Integer::from(42))),
            PatuiData::Pending(PatuiDataInner::Integer(rug::Integer::from(42))),
        ]);
        let data = data.to_known();
        assert_that!(data).is_ok();
        assert_that!(data.unwrap()).is_equal_to(PatuiDataInner::List(vec![
            PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
            PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
        ]));

        let data = PatuiDataInner::Map(
            vec![
                (
                    "key".to_string(),
                    PatuiData::Pending(PatuiDataInner::Integer(rug::Integer::from(42))),
                ),
                (
                    "key".to_string(),
                    PatuiData::Pending(PatuiDataInner::Integer(rug::Integer::from(42))),
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
                    PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
                ),
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
                ),
            ]
            .into_iter()
            .collect(),
        ));

        let data = PatuiDataInner::Map(
            vec![
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
                ),
                (
                    "key".to_string(),
                    PatuiData::Pending(PatuiDataInner::Map(
                        vec![
                            (
                                "key".to_string(),
                                PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
                            ),
                            (
                                "key".to_string(),
                                PatuiData::Pending(PatuiDataInner::Integer(rug::Integer::from(42))),
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
                    PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
                ),
                (
                    "key".to_string(),
                    PatuiData::Known(PatuiDataInner::Map(
                        vec![
                            (
                                "key".to_string(),
                                PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
                            ),
                            (
                                "key".to_string(),
                                PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
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
                PatuiData::Pending(PatuiDataInner::Integer(rug::Integer::from(42))),
            ),
            ("key2".to_string(), PatuiData::Unknown),
        ])));
        let data = data.to_known();
        assert_that!(data).is_err();
    }

    #[test]
    fn patui_list_length() {
        let inner_list = vec![
            PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
            PatuiData::Known(PatuiDataInner::Integer(rug::Integer::from(42))),
        ];

        let patui_list_data_functions = PatuiDataListMethods::new(&inner_list, true);
        let len = patui_list_data_functions.len(&vec![]);

        assert_that!(len).is_ok();
        assert_that!(len.unwrap()).is_equal_to(PatuiData::Known(PatuiDataInner::Integer(
            rug::Integer::from(2),
        )));
    }
}
