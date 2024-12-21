#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ErrorType {
    Error,
    // Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PatuiError {
    type_: ErrorType,
    display: String,
}

impl PatuiError {
    pub(crate) fn new(type_: ErrorType, display: String) -> Self {
        Self { type_, display }
    }

    // pub(crate) fn title(&self) -> &str {
    //     match self.type_ {
    //         ErrorType::Error => "Error",
    //         // ErrorType::Warning => "Warning",
    //         ErrorType::Info => "Info",
    //     }
    // }

    pub(crate) fn display(&self) -> &str {
        &self.display
    }
}
