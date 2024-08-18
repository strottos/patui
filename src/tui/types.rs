use crate::types::PatuiTest;

use super::{components::PopupComponent, error::Error};

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum MainMode {
    Test,
    TestDetail(i64),
    TestDetailSelected(i64),
}

impl MainMode {
    pub(crate) fn is_test(&self) -> bool {
        matches!(self, MainMode::Test)
    }

    pub(crate) fn is_test_detail(&self) -> bool {
        matches!(self, MainMode::TestDetail(_))
    }

    pub(crate) fn is_test_detail_selected(&self) -> bool {
        matches!(self, MainMode::TestDetailSelected(_))
    }

    pub(crate) fn matched(&self, other_mode: &MainMode) -> bool {
        match self {
            MainMode::Test => *other_mode == MainMode::Test,
            MainMode::TestDetail(_) => {
                matches!(other_mode, MainMode::TestDetail(_))
            }
            MainMode::TestDetailSelected(_) => {
                matches!(other_mode, MainMode::TestDetailSelected(_))
            }
        }
    }

    pub(crate) fn create_normal() -> Self {
        MainMode::Test
    }

    pub(crate) fn create_test_detail(id: i64) -> Self {
        MainMode::TestDetail(id)
    }

    pub(crate) fn create_test_detail_with_selected_id(id: i64) -> Self {
        MainMode::TestDetailSelected(id)
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
pub(crate) enum BreadcrumbDirection {
    Forward,
    None,
    Backward,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum PopupMode {
    CreateTest,
    UpdateTest(i64),
    Help,
}

impl PopupMode {
    pub(crate) fn title(&self) -> &str {
        match self {
            PopupMode::CreateTest => "Create Test",
            PopupMode::UpdateTest(_) => "Update Test",
            PopupMode::Help => "Help",
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum EditorMode {
    CreateTest,
    UpdateTest(i64),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum Action {
    Tick,
    Render,
    ClearKeys,
    Resize(u16, u16),
    Quit,
    Error(Error),
    ModeChange {
        mode: MainMode,
        breadcrumb_direction: BreadcrumbDirection,
    },
    PopupCreate(PopupMode),
    PopupClose,
    EditorMode(EditorMode),
    DbRead(DbRead),
    DbChange(DbChange),
}
