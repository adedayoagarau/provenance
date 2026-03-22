use super::parser;
use serde::{Deserialize, Serialize};

/// Metadata extracted from DOCX file internals — both embedded metadata and derived forensic metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxMetadata {
    // From docProps/core.xml
    pub author: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub revision_count: Option<u32>,

    // From docProps/app.xml
    pub total_editing_time_minutes: Option<u32>,
    pub application: Option<String>,
    pub app_version: Option<String>,
    pub reported_word_count: Option<u32>,
    pub reported_page_count: Option<u32>,
    pub reported_character_count: Option<u32>,
    pub reported_characters_with_spaces: Option<u32>,

    // Derived forensic metrics
    pub editing_velocity_wpm: Option<f64>,
    pub saves_per_hour: Option<f64>,
    pub creation_to_modification_minutes: Option<f64>,
    pub words_per_save: Option<f64>,

    // Flags
    pub velocity_flag: Option<String>,
    pub single_save_flag: Option<String>,
    pub rapid_creation_flag: Option<String>,
}

impl DocxMetadata {
    /// Extract metadata from DOCX archive contents and compute derived metrics.
    pub fn extract(contents: &parser::DocxArchiveContents) -> Self {
        let mut meta = DocxMetadata {
            author: None,
            created: None,
            modified: None,
            revision_count: None,
            total_editing_time_minutes: None,
            application: None,
            app_version: None,
            reported_word_count: None,
            reported_page_count: None,
            reported_character_count: None,
            reported_characters_with_spaces: None,
            editing_velocity_wpm: None,
            saves_per_hour: None,
            creation_to_modification_minutes: None,
            words_per_save: None,
            velocity_flag: None,
            single_save_flag: None,
            rapid_creation_flag: None,
        };

        // Parse core.xml
        if let Some(ref core_xml) = contents.core_xml {
            meta.author = parser::extract_xml_element_text(core_xml, "creator");
            meta.created = parser::extract_xml_element_text(core_xml, "created");
            meta.modified = parser::extract_xml_element_text(core_xml, "modified");
            meta.revision_count = parser::extract_xml_element_text(core_xml, "revision")
                .and_then(|v| v.parse().ok());
        }

        // Parse app.xml
        if let Some(ref app_xml) = contents.app_xml {
            meta.total_editing_time_minutes =
                parser::extract_xml_element_text(app_xml, "TotalTime")
                    .and_then(|v| v.parse().ok());
            meta.application = parser::extract_xml_element_text(app_xml, "Application");
            meta.app_version = parser::extract_xml_element_text(app_xml, "AppVersion");
            meta.reported_word_count = parser::extract_xml_element_text(app_xml, "Words")
                .and_then(|v| v.parse().ok());
            meta.reported_page_count = parser::extract_xml_element_text(app_xml, "Pages")
                .and_then(|v| v.parse().ok());
            meta.reported_character_count =
                parser::extract_xml_element_text(app_xml, "Characters")
                    .and_then(|v| v.parse().ok());
            meta.reported_characters_with_spaces =
                parser::extract_xml_element_text(app_xml, "CharactersWithSpaces")
                    .and_then(|v| v.parse().ok());
        }

        // Compute derived metrics
        meta.compute_derived_metrics();

        meta
    }

    fn compute_derived_metrics(&mut self) {
        // editing_velocity = word_count / total_editing_time_minutes
        if let (Some(words), Some(minutes)) =
            (self.reported_word_count, self.total_editing_time_minutes)
        {
            if minutes > 0 {
                let velocity = words as f64 / minutes as f64;
                self.editing_velocity_wpm = Some(velocity);
                if velocity > 50.0 {
                    self.velocity_flag = Some(format!(
                        "Editing velocity of {velocity:.1} words/minute exceeds 50 wpm threshold, suggesting bulk text insertion"
                    ));
                }
            }
        }

        // saves_per_hour = revision_count / (total_editing_time / 60)
        if let (Some(revisions), Some(minutes)) =
            (self.revision_count, self.total_editing_time_minutes)
        {
            if minutes > 0 {
                let hours = minutes as f64 / 60.0;
                self.saves_per_hour = Some(revisions as f64 / hours);
            }

            // Flag single-save documents with many words
            if revisions <= 1 {
                if let Some(words) = self.reported_word_count {
                    if words > 500 {
                        self.single_save_flag = Some(format!(
                            "Document contains {words} words but was saved only {revisions} time(s)"
                        ));
                    }
                }
            }
        }

        // creation_to_modification_span
        if let (Some(ref created), Some(ref modified)) = (&self.created, &self.modified) {
            if let Some(span) = compute_datetime_span_minutes(created, modified) {
                self.creation_to_modification_minutes = Some(span);
                if let Some(words) = self.reported_word_count {
                    if words > 10_000 && span < 30.0 {
                        self.rapid_creation_flag = Some(format!(
                            "Document with {words} words has a creation-to-modification span of only {span:.0} minutes"
                        ));
                    }
                }
            }
        }

        // words_per_save
        if let (Some(words), Some(revisions)) = (self.reported_word_count, self.revision_count) {
            if revisions > 0 {
                self.words_per_save = Some(words as f64 / revisions as f64);
            }
        }
    }
}

/// Parse an ISO 8601 datetime string to seconds since epoch (approximate).
fn parse_iso8601_approx(s: &str) -> Option<f64> {
    // Handle formats like "2024-01-15T10:30:00Z" or "2024-01-15T10:30:00.000Z"
    let s = s.trim().trim_end_matches('Z');

    let parts: Vec<&str> = s.splitn(2, 'T').collect();
    if parts.is_empty() {
        return None;
    }

    let date_parts: Vec<u32> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() < 3 {
        return None;
    }
    let (year, month, day) = (date_parts[0], date_parts[1], date_parts[2]);

    let mut total_seconds: f64 = 0.0;
    // Approximate days from year
    total_seconds += (year as f64 - 1970.0) * 365.25 * 86400.0;
    // Approximate days from month
    total_seconds += (month as f64 - 1.0) * 30.44 * 86400.0;
    total_seconds += (day as f64 - 1.0) * 86400.0;

    if parts.len() > 1 {
        let time_str = parts[1].split('.').next().unwrap_or("");
        let time_parts: Vec<u32> = time_str.split(':').filter_map(|p| p.parse().ok()).collect();
        if time_parts.len() >= 2 {
            total_seconds += time_parts[0] as f64 * 3600.0;
            total_seconds += time_parts[1] as f64 * 60.0;
            if time_parts.len() >= 3 {
                total_seconds += time_parts[2] as f64;
            }
        }
    }

    Some(total_seconds)
}

/// Compute the span in minutes between two ISO 8601 datetime strings.
fn compute_datetime_span_minutes(created: &str, modified: &str) -> Option<f64> {
    let created_secs = parse_iso8601_approx(created)?;
    let modified_secs = parse_iso8601_approx(modified)?;
    let diff = (modified_secs - created_secs).abs();
    Some(diff / 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso8601() {
        let secs = parse_iso8601_approx("2024-01-15T10:30:00Z");
        assert!(secs.is_some());
        assert!(secs.unwrap() > 0.0);
    }

    #[test]
    fn test_datetime_span() {
        let span =
            compute_datetime_span_minutes("2024-01-15T10:00:00Z", "2024-01-15T10:30:00Z");
        assert!(span.is_some());
        let minutes = span.unwrap();
        assert!((minutes - 30.0).abs() < 1.0);
    }

    #[test]
    fn test_velocity_flag() {
        let mut meta = DocxMetadata {
            author: None,
            created: None,
            modified: None,
            revision_count: Some(1),
            total_editing_time_minutes: Some(10),
            application: None,
            app_version: None,
            reported_word_count: Some(5000),
            reported_page_count: None,
            reported_character_count: None,
            reported_characters_with_spaces: None,
            editing_velocity_wpm: None,
            saves_per_hour: None,
            creation_to_modification_minutes: None,
            words_per_save: None,
            velocity_flag: None,
            single_save_flag: None,
            rapid_creation_flag: None,
        };
        meta.compute_derived_metrics();
        assert!(meta.velocity_flag.is_some());
        assert!(meta.editing_velocity_wpm.unwrap() > 50.0);
    }
}
