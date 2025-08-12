use crate::message::{AppMessage, SearchMessage};
use crate::search::engine;
use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::snippet::SnippetGenerator;
use tantivy::TantivyDocument;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub snippet_html: String,
}

pub fn search(
    query_str: &str,
    sender: Sender<AppMessage>,
    cancel_token: Arc<AtomicBool>,
) -> Result<()> {
    let start_time = Instant::now();

    // Lock the global index for reading.
    let index_lock = match engine::INDEX.read() {
        Ok(guard) => guard,
        Err(_poisoned) => {
            sender.send(AppMessage::Search(SearchMessage::Error(
                "Index lock poisoned. Please restart or re-index.".to_string(),
            )))?;
            return Ok(());
        }
    };
    let Some((index, reader)) = &*index_lock else {
        sender.send(AppMessage::Search(SearchMessage::Error(
            "Index not found. Please index a directory first.".to_string(),
        )))?;
        return Ok(());
    };

    let searcher = reader.searcher();
    let schema = index.schema();

    let path_field = schema.get_field("path").context("Schema error: 'path' field not found")?;
    let content_field = schema.get_field("content").context("Schema error: 'content' field not found")?;

    let query_parser = QueryParser::for_index(index, vec![content_field]);
    let query = query_parser.parse_query(query_str)?;

    // Reduced the search limit to 100 for performance and stability.
    let top_docs = searcher.search(&query, &TopDocs::with_limit(100))?;

    if top_docs.is_empty() {
        sender.send(AppMessage::Search(SearchMessage::Finished {
            results: Vec::new(), // Send a truly empty vector for no results.
            duration: start_time.elapsed(),
        }))?;
        return Ok(());
    }

    let mut snippet_generator = SnippetGenerator::create(&searcher, &query, content_field)?;
    snippet_generator.set_max_num_chars(120);

    let mut results = Vec::new();
    for (_score, doc_address) in top_docs {
        // Check for cancellation signal periodically.
        if cancel_token.load(Ordering::SeqCst) {
            sender.send(AppMessage::Search(SearchMessage::Cancelled))?;
            return Ok(());
        }

        let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
        let snippet = snippet_generator.snippet_from_doc(&retrieved_doc);

        let path = retrieved_doc
            .get_first(path_field)
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Path")
            .to_string();

        results.push(SearchResult {
            path,
            snippet_html: snippet.to_html(),
        });
    }

    sender.send(AppMessage::Search(SearchMessage::Finished {
        results,
        duration: start_time.elapsed(),
    }))?;

    Ok(())
}
