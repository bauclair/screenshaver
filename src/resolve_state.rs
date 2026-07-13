#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScreenState {
    Active,
    Idle,
}

pub fn resolve_state(activity: bool, idle: bool) -> ScreenState {
    if activity {
        ScreenState::Active
    } else if idle {
        ScreenState::Idle
    } else {
        ScreenState::Active
    }
}