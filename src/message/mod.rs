use crate::config::Theme;
use crate::search::query::SearchResult;
use std::time::Duration;

// --- Module-specific messages ---

#[derive(Debug)]
pub enum IndexMessage {
    Progress(f32),
    Finished,
    Error(String),
}

#[derive(Debug)]
pub enum SearchMessage {
    Finished {
        results: Vec<SearchResult>,
        duration: Duration,
    },
    Cancelled,
    Error(String),
}

#[derive(Debug)]
pub enum SettingsMessage {
    ThemeChanged(Theme),
}

// --- Top-level message router ---
#[derive(Debug)]
pub enum AppMessage {
    Index(IndexMessage),
    Search(SearchMessage),
    Settings(SettingsMessage),
}