use crate::types::{PatuiTest, PatuiTestDetails, PatuiTestId, PatuiTestStepId};
use crossterm::event::KeyEvent;
use eyre::Result;

use super::{error::Error, popups::PopupComponent};

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub(crate) enum Mode {
    #[default]
    TestList,
    TestListWithDetails,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum DbRead {
    Test,
    TestDetail(PatuiTestId),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum DbCreate {
    Test(PatuiTestDetails),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum DbUpdate {
    Test(PatuiTest),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum UpdateData {
    Tests(Vec<PatuiTest>),
    TestDetail(PatuiTest),
    BreadcrumbTitles(Vec<String>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
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

pub(crate) trait Component: std::fmt::Debug {
    /// Take input for the component and optionally send back an action to perform
    fn input(&mut self, _key: &KeyEvent, _mode: &PaneType) -> Result<Vec<Action>> {
        Ok(vec![])
    }

    /// Get the keys that the component is listening for
    fn keys(&self, _mode: &PaneType) -> Vec<HelpItem> {
        vec![]
    }

    /// Update the component based on an action and optionally send back actions to perform
    fn update(&mut self, _action: &Action) -> Result<Vec<Action>> {
        Ok(vec![])
    }
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum EditorMode {
    CreateTest,
    UpdateTest(PatuiTestId),
    UpdateTestStep(PatuiTestId, PatuiTestStepId),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum Action {
    Tick,
    Render,
    ClearKeys,
    Resize(u16, u16),
    Quit,
    ForceRedraw,
    Error(Error),
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
