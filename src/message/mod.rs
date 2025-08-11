use crate::config::Theme;
use crate::search::query::SearchResult;

// --- Module-specific messages ---

#[derive(Debug)]
pub enum IndexMessage {
    Progress(f32),
    Finished,
    Error(String),
}

#[derive(Debug)]
pub enum SearchMessage {
    Finished(Vec<SearchResult>),
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