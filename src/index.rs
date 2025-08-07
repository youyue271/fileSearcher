use crate::app_message::{AppMessage, IndexMessage};
use anyhow::Result;
use crossbeam_channel::Sender;
use docx_rust::document::{BodyContent, ParagraphContent, RunContent};
use docx_rust::DocxFile;
use std::path::Path;
use tantivy::directory::MmapDirectory;
use tantivy::schema::*;
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{doc, Index};
use tantivy_jieba::JiebaTokenizer;
use walkdir::WalkDir;

const INDEX_DIR: &str = "tantivy_index";

// Docx文本提取
fn extract_text_from_docx(path: &Path) -> Result<String> {
    let docx = DocxFile::from_file(path)?;
    let mut docx = docx.parse()?;
    let mut text = String::new();

    for content in std::mem::take(&mut docx.document.body.content) {
        if let BodyContent::Paragraph(p) = content {
            for run in p.content {
                if let ParagraphContent::Run(r) = run {
                    for text_content in r.content {
                        if let RunContent::Text(t) = text_content {
                            text.push_str(&t.text);
                        }
                    }
                }
            }
            text.push('\n');
        }
    }
    Ok(text)
}

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
                match extract_text_from_docx(file_path) {
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

    // 5. Send finished signal
    sender.send(AppMessage::Index(IndexMessage::Finished))?;

    Ok(())
}
