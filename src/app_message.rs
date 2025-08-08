
use crate::search::SearchResult;

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

// --- Top-level message router ---
#[derive(Debug)]
pub enum AppMessage {
    Index(IndexMessage),
    Search(SearchMessage),
}
