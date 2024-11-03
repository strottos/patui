#[derive(Debug)]
pub(crate) enum ScrollType {
    Single(isize),
    HalfPageUp,
    HalfPageDown,
    HalfPageLeft,
    HalfPageRight,
    FullPageUp,
    FullPageDown,
    Top,
    Bottom,
}
