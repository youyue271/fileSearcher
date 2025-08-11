use anyhow::Result;
use docx_rust::document::{BodyContent, ParagraphContent, RunContent};
use docx_rust::DocxFile;
use std::path::Path;

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

pub fn read_file_content(path: &Path) -> Result<String> {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if ext.eq_ignore_ascii_case("docx") {
            return extract_text_from_docx(path);
        }
    }

    // Fallback for plain text files
    std::fs::read_to_string(path).map_err(anyhow::Error::from)
}