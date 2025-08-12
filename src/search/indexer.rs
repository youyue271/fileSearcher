use crate::message::{AppMessage, IndexMessage};
use crate::search::engine;
use crate::utils::file_utils;
use anyhow::Result;
use crossbeam_channel::Sender;
use std::path::Path;
use tantivy::directory::MmapDirectory;
use tantivy::schema::*;
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{doc, Index};
use tantivy_jieba::JiebaTokenizer;
use walkdir::WalkDir;

const INDEX_DIR: &str = "tantivy_index";

pub fn index_directory(path: &Path, sender: Sender<AppMessage>) -> Result<()> {
    println!("Starting indexing process for: {:?}", path);

    // 1. Count total files for progress tracking
    let walker = WalkDir::new(path).into_iter();
    let total_files = walker
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("docx"))
        })
        .count();
    let mut processed_files = 0;

    // 2. Setup Tantivy Index
    let index_path = Path::new(INDEX_DIR);
    std::fs::create_dir_all(index_path)?;
    let directory = MmapDirectory::open(index_path)?;

    let mut schema_builder = Schema::builder();
    let path_field = schema_builder.add_text_field("path", TEXT | STORED);

    let text_indexing = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("jieba")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    let content_field = schema_builder.add_text_field("content", text_indexing);
    let schema = schema_builder.build();
    let index = Index::open_or_create(directory, schema.clone())?;
    index
        .tokenizers()
        .register("jieba", TextAnalyzer::from(JiebaTokenizer {}));
    let mut index_writer = index.writer(50_000_000)?;
    // Clear old index data
    index_writer.delete_all_documents()?;

    // 3. Process and index each file
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        if let Some(extension) = file_path.extension().and_then(|s| s.to_str()) {
            if extension.eq_ignore_ascii_case("docx") {
                println!("Indexing: {:?}", file_path);
                match file_utils::read_file_content(file_path) {
                    Ok(content) => {
                        if !content.is_empty() {
                            index_writer.add_document(doc!(
                                path_field => file_path.to_str().unwrap_or_default(),
                                content_field => content
                            ))?;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to process file {:?}: {}", file_path, e);
                    }
                }
                // 4. Update progress
                processed_files += 1;
                let progress = if total_files > 0 {
                    processed_files as f32 / total_files as f32
                } else {
                    1.0 // Avoid division by zero
                };
                // Send progress back to the UI thread
                sender
                    .send(AppMessage::Index(IndexMessage::Progress(progress)))
                    .unwrap();
            }
        }
    }

    index_writer.commit()?;
    println!("Indexing completed successfully.");

    // After commit, load the index and reader into our static variable.
    let reader = index.reader_builder().try_into()?;
    let mut index_lock = match engine::INDEX.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *index_lock = Some((index, reader));
    println!("Index and reader loaded into memory.");

    // 5. Send finished signal
    sender.send(AppMessage::Index(IndexMessage::Finished))?;

    Ok(())
}
