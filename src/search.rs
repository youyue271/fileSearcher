use crate::app_message::{AppMessage, SearchMessage};
use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::snippet::SnippetGenerator;
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{Index, TantivyDocument};
use tantivy_jieba::JiebaTokenizer;

const INDEX_DIR: &str = "tantivy_index";

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub snippet_html: String,
}

pub fn search(query_str: &str, sender: Sender<AppMessage>) -> Result<()> {
    let index_path = Path::new(INDEX_DIR);
    if !index_path.exists() {
        sender.send(AppMessage::Search(SearchMessage::Error(
            "Index not found. Please index a directory first.".to_string(),
        )))?;
        return Ok(());
    }

    let directory = MmapDirectory::open(index_path)?;
    let index = Index::open(directory)?;

    index
        .tokenizers()
        .register("jieba", TextAnalyzer::from(JiebaTokenizer {}));

    let reader = index.reader()?;
    let searcher = reader.searcher();
    let schema = index.schema();

    let path_field = schema.get_field("path").context("Schema error: 'path' field not found")?;
    let content_field = schema.get_field("content").context("Schema error: 'content' field not found")?;

    let query_parser = QueryParser::for_index(&index, vec![content_field]);
    let query = query_parser.parse_query(query_str)?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(20))?;

    if top_docs.is_empty() {
        let results = vec![SearchResult {
            path: "No documents found matching your query.".to_string(),
            snippet_html: "".to_string(),
        }];
        sender.send(AppMessage::Search(SearchMessage::Finished(results)))?;
        return Ok(());
    }

    let mut snippet_generator = SnippetGenerator::create(&searcher, &query, content_field)?;
    snippet_generator.set_max_num_chars(120);

    let mut results = Vec::new();
    for (_score, doc_address) in top_docs {
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

    sender.send(AppMessage::Search(SearchMessage::Finished(results)))?;

    Ok(())
}
