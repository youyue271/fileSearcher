
#[derive(PartialEq, Debug)]
pub enum AppState {
    Idle,
    Indexing { progress: f32 },
    Searching,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Idle
    }
}
