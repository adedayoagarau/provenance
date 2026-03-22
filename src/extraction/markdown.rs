use crate::utils::errors::{ProvenanceError, Result};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::Path;

/// Extract plain text from a Markdown file, stripping all markup.
pub fn extract(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(strip_markdown(&content))
}

/// Strip Markdown syntax and return plain text.
pub fn strip_markdown(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut output = String::with_capacity(markdown.len());
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Text(text) => {
                if !in_code_block {
                    output.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                output.push('\n');
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                output.push_str("\n\n");
            }
            Event::Start(Tag::Heading { .. }) => {}
            Event::End(TagEnd::Heading(_)) => {
                output.push_str("\n\n");
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::Start(Tag::Item) => {}
            Event::End(TagEnd::Item) => {
                output.push('\n');
            }
            Event::Code(code) => {
                // Inline code — include the text content
                output.push_str(&code);
            }
            _ => {}
        }
    }

    output
}
