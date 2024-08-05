#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ErrorType {
    Error,
    // Warning,
    Info,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    type_: ErrorType,
    display: String,
}

impl Error {
    pub fn new(type_: ErrorType, display: String) -> Self {
        Self { type_, display }
    }

    pub fn title(&self) -> &str {
        match self.type_ {
            ErrorType::Error => "Error",
            // ErrorType::Warning => "Warning",
            ErrorType::Info => "Info",
        }
    }

    pub fn display(&self) -> &str {
        &self.display
    }
}
