use crate::types::PatuiTest;
use color_eyre::Result;
use crossterm::event::KeyEvent;

use super::{error::Error, popups::PopupComponent};

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub(crate) enum PaneType {
    #[default]
    Test,
    TestDetail(i64),
    TestDetailSelected(i64),
    TestDetailStep(i64, usize),
}

impl PaneType {
    pub(crate) fn is_test(&self) -> bool {
        matches!(self, PaneType::Test)
    }

    pub(crate) fn is_test_detail(&self) -> bool {
        matches!(self, PaneType::TestDetail(_))
    }

    pub(crate) fn is_test_detail_selected(&self) -> bool {
        matches!(self, PaneType::TestDetailSelected(_))
    }

    pub(crate) fn is_test_detail_step(&self) -> bool {
        matches!(self, PaneType::TestDetailStep(_, _))
    }

    pub(crate) fn matched(&self, other_mode: &PaneType) -> bool {
        match self {
            PaneType::Test => *other_mode == PaneType::Test,
            PaneType::TestDetail(_) => {
                matches!(other_mode, PaneType::TestDetail(_))
            }
            PaneType::TestDetailSelected(_) => {
                matches!(other_mode, PaneType::TestDetailSelected(_))
            }
            PaneType::TestDetailStep(_, _) => matches!(other_mode, PaneType::TestDetailStep(_, _)),
        }
    }

    pub(crate) fn create_normal() -> Self {
        PaneType::Test
    }

    pub(crate) fn create_test_detail(id: i64) -> Self {
        PaneType::TestDetail(id)
    }

    pub(crate) fn create_test_detail_with_selected_id(id: i64) -> Self {
        PaneType::TestDetailSelected(id)
    }

    pub(crate) fn create_test_detail_step(id: i64, step_num: usize) -> Self {
        PaneType::TestDetailStep(id, step_num)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum DbRead {
    Test,
    TestDetail(i64),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum DbChange {
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
    UpdateTest(i64),
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
    UpdateTest(i64),
    UpdateTestStep(i64, usize),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum Action {
    Tick,
    Render,
    ClearKeys,
    Resize(u16, u16),
    Quit,
    Error(Error),
    ModeChange { mode: PaneType },
    PaneChange(usize),
    PopupCreate(PopupMode),
    PopupClose,
    EditorMode(EditorMode),
    DbRead(DbRead),
    DbChange(DbChange),
    UpdateData(UpdateData),
}
