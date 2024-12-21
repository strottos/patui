use crate::db::{PatuiTestDb, PatuiTestId};
use crate::types::{PatuiTest, PatuiTestDetails};

use super::{error::PatuiError, popups::PopupComponent};

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) enum Mode {
    #[default]
    TestList,
    TestListWithDetails,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) enum StatusChange {
    #[default]
    Reset,
    ModeChangeTestList,
    ModeChangeTestListWithDetails(PatuiTestId),
}

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) enum PaneType {
    #[default]
    TestList,
    TestDetail,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DbRead {
    Test,
    TestDetail(PatuiTestId),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DbCreate {
    Test(PatuiTestDetails),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DbUpdate {
    Test(PatuiTest),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UpdateData {
    Tests(Vec<PatuiTestDb>),
    TestDetail(PatuiTest),
    // BreadcrumbTitles(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PopupMode {
    CreateTest,
    UpdateTest(PatuiTestId),
    Help,
    Error,
}

impl PopupMode {
    pub(crate) fn title(&self) -> &str {
        match self {
            PopupMode::CreateTest => "Create Test",
            PopupMode::UpdateTest(_) => "Update Test",
            PopupMode::Help => "Help",
            PopupMode::Error => "Error",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Popup {
    pub(crate) mode: PopupMode,
    pub(crate) component: Box<dyn PopupComponent>,
}

impl Popup {
    pub(crate) fn new(mode: PopupMode, component: Box<dyn PopupComponent>) -> Self {
        Self { mode, component }
    }
}

#[derive(Debug)]
pub(crate) struct HelpItem {
    pub(crate) keys: &'static str,
    pub(crate) minidesc: &'static str,
    pub(crate) desc: &'static str,
}

impl HelpItem {
    pub(crate) fn new(keys: &'static str, minidesc: &'static str, desc: &'static str) -> Self {
        Self {
            keys,
            minidesc,
            desc,
        }
    }

    pub(crate) fn bottom_bar_help(&self) -> String {
        format!("{}: {}", self.keys, self.minidesc)
    }

    pub(crate) fn global_help(&self) -> String {
        format!("{}: {}", self.keys, self.desc)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EditorMode {
    CreateTest,
    UpdateTest(PatuiTestId),
    // UpdateTestStep(PatuiTestId, PatuiTestStepId),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Action {
    Tick,
    Render,
    ClearKeys,
    Resize(u16, u16),
    Quit,
    ForceRedraw,
    Error(PatuiError),
    StatusChange(StatusChange),
    PaneChange(PaneType),
    PopupCreate(PopupMode),
    PopupClose,
    EditorMode(EditorMode),
    DbRead(DbRead),
    DbCreate(DbCreate),
    DbUpdate(DbUpdate),
    UpdateData(UpdateData),
}
