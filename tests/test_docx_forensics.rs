use std::io::Write;
use std::path::PathBuf;

/// Helper to create a minimal .docx file (ZIP archive with Word XML) for testing.
struct DocxBuilder {
    paragraphs: Vec<DocxParagraph>,
    core_xml: Option<String>,
    app_xml: Option<String>,
    settings_rsids: Vec<String>,
    styles_xml: Option<String>,
}

struct DocxParagraph {
    rsid_r: String,
    rsid_r_default: Option<String>,
    runs: Vec<DocxRun>,
}

struct DocxRun {
    text: String,
    rsid_r: Option<String>,
    font_name: Option<String>,
    font_size: Option<u32>,
    bold: bool,
}

impl DocxBuilder {
    fn new() -> Self {
        DocxBuilder {
            paragraphs: Vec::new(),
            core_xml: None,
            app_xml: None,
            settings_rsids: Vec::new(),
            styles_xml: None,
        }
    }

    fn add_paragraph(&mut self, rsid: &str, text: &str) -> &mut Self {
        self.paragraphs.push(DocxParagraph {
            rsid_r: rsid.to_string(),
            rsid_r_default: Some(rsid.to_string()),
            runs: vec![DocxRun {
                text: text.to_string(),
                rsid_r: Some(rsid.to_string()),
                font_name: None,
                font_size: None,
                bold: false,
            }],
        });
        self
    }

    fn add_paragraph_with_run(
        &mut self,
        rsid: &str,
        runs: Vec<(&str, Option<&str>, Option<u32>, bool)>,
    ) -> &mut Self {
        self.paragraphs.push(DocxParagraph {
            rsid_r: rsid.to_string(),
            rsid_r_default: Some(rsid.to_string()),
            runs: runs
                .into_iter()
                .map(|(text, font, size, bold)| DocxRun {
                    text: text.to_string(),
                    rsid_r: Some(rsid.to_string()),
                    font_name: font.map(String::from),
                    font_size: size,
                    bold,
                })
                .collect(),
        });
        self
    }

    fn set_core_metadata(
        &mut self,
        author: &str,
        created: &str,
        modified: &str,
        revision: u32,
    ) -> &mut Self {
        self.core_xml = Some(format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:dcterms="http://purl.org/dc/terms/"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <dc:creator>{author}</dc:creator>
    <dcterms:created xsi:type="dcterms:W3CDTF">{created}</dcterms:created>
    <dcterms:modified xsi:type="dcterms:W3CDTF">{modified}</dcterms:modified>
    <cp:revision>{revision}</cp:revision>
</cp:coreProperties>"#
        ));
        self
    }

    fn set_app_metadata(
        &mut self,
        application: &str,
        total_time: u32,
        words: u32,
        pages: u32,
    ) -> &mut Self {
        self.app_xml = Some(format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
    <Application>{application}</Application>
    <TotalTime>{total_time}</TotalTime>
    <Words>{words}</Words>
    <Pages>{pages}</Pages>
    <Characters>{}</Characters>
    <CharactersWithSpaces>{}</CharactersWithSpaces>
</Properties>"#,
            words * 5,
            words * 6
        ));
        self
    }

    fn set_settings_rsids(&mut self, rsids: &[&str]) -> &mut Self {
        self.settings_rsids = rsids.iter().map(|s| s.to_string()).collect();
        self
    }

    fn set_styles_xml(
        &mut self,
        default_font: &str,
        default_size: u32,
        default_lang: &str,
    ) -> &mut Self {
        self.styles_xml = Some(format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:docDefaults>
        <w:rPrDefault>
            <w:rPr>
                <w:rFonts w:ascii="{default_font}" w:hAnsi="{default_font}"/>
                <w:sz w:val="{default_size}"/>
                <w:lang w:val="{default_lang}"/>
            </w:rPr>
        </w:rPrDefault>
    </w:docDefaults>
</w:styles>"#
        ));
        self
    }

    fn build_document_xml(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
"#,
        );

        for para in &self.paragraphs {
            let rsid_default = para
                .rsid_r_default
                .as_deref()
                .unwrap_or(&para.rsid_r);
            xml.push_str(&format!(
                r#"<w:p w:rsidR="{}" w:rsidRDefault="{}" w:rsidP="{}">"#,
                para.rsid_r, rsid_default, para.rsid_r
            ));

            for run in &para.runs {
                let rsid_attr = run
                    .rsid_r
                    .as_deref()
                    .map(|r| format!(r#" w:rsidR="{}""#, r))
                    .unwrap_or_default();
                xml.push_str(&format!(r#"<w:r{rsid_attr}>"#));

                // Run properties
                let has_rpr = run.font_name.is_some() || run.font_size.is_some() || run.bold;
                if has_rpr {
                    xml.push_str("<w:rPr>");
                    if let Some(ref font) = run.font_name {
                        xml.push_str(&format!(
                            r#"<w:rFonts w:ascii="{font}" w:hAnsi="{font}"/>"#
                        ));
                    }
                    if let Some(size) = run.font_size {
                        xml.push_str(&format!(r#"<w:sz w:val="{size}"/>"#));
                    }
                    if run.bold {
                        xml.push_str("<w:b/>");
                    }
                    xml.push_str("</w:rPr>");
                }

                xml.push_str(&format!(
                    r#"<w:t xml:space="preserve">{}</w:t>"#,
                    run.text
                ));
                xml.push_str("</w:r>");
            }

            xml.push_str("</w:p>\n");
        }

        xml.push_str("</w:body>\n</w:document>");
        xml
    }

    fn build_settings_xml(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:rsids>
"#,
        );
        for rsid in &self.settings_rsids {
            xml.push_str(&format!(r#"<w:rsid w:val="{rsid}"/>"#));
            xml.push('\n');
        }
        xml.push_str("</w:rsids>\n</w:settings>");
        xml
    }

    fn write_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", options)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
        )?;

        // _rels/.rels
        zip.start_file("_rels/.rels", options)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
        )?;

        // word/document.xml
        zip.start_file("word/document.xml", options)?;
        zip.write_all(self.build_document_xml().as_bytes())?;

        // docProps/core.xml
        if let Some(ref core) = self.core_xml {
            zip.start_file("docProps/core.xml", options)?;
            zip.write_all(core.as_bytes())?;
        }

        // docProps/app.xml
        if let Some(ref app) = self.app_xml {
            zip.start_file("docProps/app.xml", options)?;
            zip.write_all(app.as_bytes())?;
        }

        // word/settings.xml
        if !self.settings_rsids.is_empty() {
            zip.start_file("word/settings.xml", options)?;
            zip.write_all(self.build_settings_xml().as_bytes())?;
        }

        // word/styles.xml
        if let Some(ref styles) = self.styles_xml {
            zip.start_file("word/styles.xml", options)?;
            zip.write_all(styles.as_bytes())?;
        }

        zip.finish()?;
        Ok(())
    }
}

fn temp_docx(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("provenance_docx_tests");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(name)
}

// ============================================================================
// 12.1 & 12.2: Metadata extraction tests
// ============================================================================

#[test]
fn test_docx_metadata_extraction() {
    let path = temp_docx("meta_test.docx");
    let mut builder = DocxBuilder::new();
    builder
        .set_core_metadata(
            "Alice Author",
            "2024-06-15T10:00:00Z",
            "2024-06-20T14:30:00Z",
            12,
        )
        .set_app_metadata("Microsoft Office Word", 480, 3500, 15)
        .add_paragraph("00AA1111", "This is a test paragraph.");

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    let meta = &profile.metadata;

    assert_eq!(meta.author.as_deref(), Some("Alice Author"));
    assert_eq!(meta.revision_count, Some(12));
    assert_eq!(meta.total_editing_time_minutes, Some(480));
    assert_eq!(meta.reported_word_count, Some(3500));
    assert_eq!(meta.reported_page_count, Some(15));
    assert!(meta.application.as_deref().unwrap().contains("Word"));

    // Derived metrics
    let velocity = meta.editing_velocity_wpm.unwrap();
    assert!((velocity - 7.29).abs() < 0.1, "velocity = {velocity}");
    assert!(meta.velocity_flag.is_none()); // 7.29 wpm is normal

    let saves_per_hour = meta.saves_per_hour.unwrap();
    assert!(saves_per_hour > 0.0);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_velocity_flag_triggered() {
    let path = temp_docx("velocity_flag.docx");
    let mut builder = DocxBuilder::new();
    builder
        .set_core_metadata("Writer", "2024-01-01T10:00:00Z", "2024-01-01T10:10:00Z", 1)
        .set_app_metadata("Microsoft Office Word", 10, 5000, 20)
        .add_paragraph("00AA1111", "Bulk pasted text.");

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    assert!(
        profile.metadata.velocity_flag.is_some(),
        "Should flag 500 wpm velocity"
    );
    assert!(profile.metadata.editing_velocity_wpm.unwrap() > 50.0);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_single_save_flag() {
    let path = temp_docx("single_save.docx");
    let mut builder = DocxBuilder::new();
    builder
        .set_core_metadata("Writer", "2024-01-01T10:00:00Z", "2024-01-01T10:05:00Z", 1)
        .set_app_metadata("Microsoft Office Word", 5, 3000, 10)
        .add_paragraph("00AA1111", "Content.");

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    assert!(
        profile.metadata.single_save_flag.is_some(),
        "Should flag single-save document with 3000 words"
    );

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// 12.3: RSID analysis tests
// ============================================================================

#[test]
fn test_rsid_diversity_organic() {
    let path = temp_docx("rsid_organic.docx");
    let mut builder = DocxBuilder::new();

    // Simulate organic writing: many different rsids across paragraphs
    let rsids = [
        "00AA1111", "00BB2222", "00CC3333", "00DD4444", "00EE5555",
        "00FF6666", "01117777", "01228888", "01339999", "0144AAAA",
        "0155BBBB", "0166CCCC", "0177DDDD", "0188EEEE", "0199FFFF",
    ];
    for (i, rsid) in rsids.iter().enumerate() {
        builder.add_paragraph(rsid, &format!("Paragraph {i} with unique session."));
    }
    builder.set_settings_rsids(&rsids);
    builder.set_core_metadata("Author", "2024-01-01T09:00:00Z", "2024-03-15T16:00:00Z", 42);
    builder.set_app_metadata("Microsoft Office Word", 1200, 5000, 15);

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    let rsid_analysis = &profile.rsid_analysis;

    assert!(
        rsid_analysis.rsid_diversity > 0.3,
        "Organic diversity should be > 0.3, got {}",
        rsid_analysis.rsid_diversity
    );
    assert_eq!(rsid_analysis.unique_rsid_count, 15);
    assert!(rsid_analysis.largest_block_size <= 1);
    assert!(rsid_analysis.paste_events.is_empty());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_rsid_diversity_bulk_paste() {
    let path = temp_docx("rsid_bulk.docx");
    let mut builder = DocxBuilder::new();

    // Simulate bulk paste: all paragraphs share one rsid
    for i in 0..25 {
        builder.add_paragraph("00AA1111", &format!("Pasted paragraph {i} from AI output."));
    }
    builder.set_settings_rsids(&["00AA1111"]);
    builder.set_core_metadata("Writer", "2024-01-01T10:00:00Z", "2024-01-01T10:05:00Z", 1);
    builder.set_app_metadata("Microsoft Office Word", 5, 2500, 10);

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    let rsid_analysis = &profile.rsid_analysis;

    assert!(
        rsid_analysis.rsid_diversity < 0.05,
        "Bulk paste diversity should be < 0.05, got {}",
        rsid_analysis.rsid_diversity
    );
    assert_eq!(rsid_analysis.unique_rsid_count, 1);
    assert_eq!(rsid_analysis.largest_block_size, 25);
    assert!(
        rsid_analysis
            .anomalies
            .iter()
            .any(|a| a.anomaly_type == "LowRsidDiversity"),
        "Should flag low RSID diversity"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_rsid_clustering() {
    let path = temp_docx("rsid_clustering.docx");
    let mut builder = DocxBuilder::new();

    // Three blocks of different rsids
    for _ in 0..5 {
        builder.add_paragraph("AAAA1111", "Block A paragraph.");
    }
    for _ in 0..3 {
        builder.add_paragraph("BBBB2222", "Block B paragraph.");
    }
    for _ in 0..7 {
        builder.add_paragraph("CCCC3333", "Block C paragraph.");
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    let blocks = &profile.rsid_analysis.rsid_blocks;

    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].block_length, 5);
    assert_eq!(blocks[0].rsid, "AAAA1111");
    assert_eq!(blocks[1].block_length, 3);
    assert_eq!(blocks[1].rsid, "BBBB2222");
    assert_eq!(blocks[2].block_length, 7);
    assert_eq!(blocks[2].rsid, "CCCC3333");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_rsid_progression() {
    let path = temp_docx("rsid_progression.docx");
    let mut builder = DocxBuilder::new();

    // RSIDs increase through the document (sequential writing)
    for i in 0..10 {
        let rsid = format!("{:08X}", 0x00100000 + i * 0x00010000);
        builder.add_paragraph(&rsid, &format!("Sequential paragraph {i}."));
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    let correlation = profile.rsid_analysis.rsid_progression_correlation;

    assert!(
        correlation.is_some(),
        "Should compute progression correlation"
    );
    assert!(
        correlation.unwrap() > 0.5,
        "Sequential RSIDs should have positive correlation, got {:?}",
        correlation
    );

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// 12.4: Formatting consistency tests
// ============================================================================

#[test]
fn test_formatting_consistency_clean() {
    let path = temp_docx("fmt_clean.docx");
    let mut builder = DocxBuilder::new();

    builder.set_styles_xml("Calibri", 22, "en-US");

    // All runs use baseline formatting
    for i in 0..5 {
        builder.add_paragraph_with_run(
            &format!("{:08X}", i),
            vec![(&format!("Paragraph {i} text."), Some("Calibri"), Some(22), false)],
        );
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    assert!(
        profile.formatting_analysis.formatting_consistency_score >= 0.9,
        "Clean doc should have high consistency, got {}",
        profile.formatting_analysis.formatting_consistency_score
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_formatting_consistency_paste_artifacts() {
    let path = temp_docx("fmt_paste.docx");
    let mut builder = DocxBuilder::new();

    builder.set_styles_xml("Calibri", 22, "en-US");

    // First paragraphs use baseline
    for i in 0..3 {
        builder.add_paragraph_with_run(
            &format!("{:08X}", i),
            vec![(&format!("Native paragraph {i}."), Some("Calibri"), Some(22), false)],
        );
    }

    // Pasted paragraphs with different font
    for i in 3..8 {
        builder.add_paragraph_with_run(
            "PASTE_ID",
            vec![(
                &format!("Pasted paragraph {i} from external source."),
                Some("Times New Roman"),
                Some(24),
                false,
            )],
        );
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    assert!(
        profile.formatting_analysis.formatting_consistency_score < 0.9,
        "Paste artifacts should lower consistency, got {}",
        profile.formatting_analysis.formatting_consistency_score
    );
    assert!(
        !profile.formatting_analysis.anomalies.is_empty(),
        "Should detect formatting anomalies"
    );

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// 12.5: Structural analysis tests
// ============================================================================

#[test]
fn test_structural_varied_paragraphs() {
    let path = temp_docx("struct_varied.docx");
    let mut builder = DocxBuilder::new();

    let lengths = [
        "Short.",
        "A medium length sentence that has more words in it.",
        "This is a much longer paragraph that contains many words and goes on for quite a while to simulate the kind of varied paragraph lengths you would see in organic human writing where some thoughts are brief and others require extended explanation.",
        "Another short one.",
        "The final paragraph is moderately sized with enough words to provide variety.",
    ];
    for (i, text) in lengths.iter().enumerate() {
        builder.add_paragraph(&format!("{:08X}", i), text);
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    assert!(
        profile.structural_forensics.coefficient_of_variation > 0.3,
        "Varied paragraphs should have high CV, got {}",
        profile.structural_forensics.coefficient_of_variation
    );

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// 12.6: Construction profile tests
// ============================================================================

#[test]
fn test_construction_profile_organic() {
    let path = temp_docx("profile_organic.docx");
    let mut builder = DocxBuilder::new();

    builder.set_core_metadata(
        "Jane Doe",
        "2024-01-15T09:00:00Z",
        "2024-02-28T16:30:00Z",
        35,
    );
    builder.set_app_metadata("Microsoft Office Word", 900, 4500, 18);
    builder.set_styles_xml("Calibri", 22, "en-US");

    // Many diverse sessions with revision
    let rsids: Vec<String> = (0..20)
        .map(|i| format!("{:08X}", 0x00100000 + i * 0x00050000))
        .collect();
    let rsid_refs: Vec<&str> = rsids.iter().map(|s| s.as_str()).collect();
    builder.set_settings_rsids(&rsid_refs);

    for (i, rsid) in rsids.iter().enumerate() {
        let text = if i % 3 == 0 {
            "A short thought.".to_string()
        } else if i % 3 == 1 {
            format!("A medium-length paragraph that explores idea {i} with some detail and nuance.")
        } else {
            format!(
                "This is a longer paragraph discussing topic {i} in considerable depth, \
                 covering multiple aspects and providing examples. The writer clearly spent \
                 time thinking about this section before committing it to the page."
            )
        };
        builder.add_paragraph_with_run(
            rsid,
            vec![(&text, Some("Calibri"), Some(22), false)],
        );
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();

    assert_eq!(
        profile.construction_pattern,
        provenance::forensics::docx::profile::ConstructionPattern::Organic,
        "Should classify as Organic, got {:?}",
        profile.construction_pattern
    );
    assert!(profile.process_integrity_score > 0.5);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_construction_profile_bulk() {
    let path = temp_docx("profile_bulk.docx");
    let mut builder = DocxBuilder::new();

    builder.set_core_metadata(
        "Student",
        "2024-03-01T23:00:00Z",
        "2024-03-01T23:05:00Z",
        1,
    );
    builder.set_app_metadata("Microsoft Office Word", 5, 4000, 12);
    builder.set_settings_rsids(&["00AA1111"]);

    // All paragraphs share one rsid — bulk paste
    for i in 0..30 {
        builder.add_paragraph(
            "00AA1111",
            &format!(
                "This is paragraph {i} of the essay that was pasted all at once."
            ),
        );
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();

    assert_eq!(
        profile.construction_pattern,
        provenance::forensics::docx::profile::ConstructionPattern::BulkInsertion,
        "Should classify as BulkInsertion, got {:?}",
        profile.construction_pattern
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_construction_profile_hybrid() {
    let path = temp_docx("profile_hybrid.docx");
    let mut builder = DocxBuilder::new();

    builder.set_core_metadata(
        "Writer",
        "2024-01-10T08:00:00Z",
        "2024-01-20T15:00:00Z",
        15,
    );
    builder.set_app_metadata("Microsoft Office Word", 300, 3000, 10);

    // First half: organic (diverse rsids)
    for i in 0..10 {
        builder.add_paragraph(
            &format!("{:08X}", 0x00100000 + i * 0x00050000),
            &format!("Organically written paragraph {i}."),
        );
    }

    // Second half: bulk paste (one rsid for many paragraphs)
    for i in 10..30 {
        builder.add_paragraph(
            "BULK_RSID",
            &format!("Bulk pasted paragraph {i}."),
        );
    }

    let mut rsids: Vec<String> = (0..10)
        .map(|i| format!("{:08X}", 0x00100000 + i * 0x00050000))
        .collect();
    rsids.push("BULK_RSID".to_string());
    let rsid_refs: Vec<&str> = rsids.iter().map(|s| s.as_str()).collect();
    builder.set_settings_rsids(&rsid_refs);

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();

    // Should be Hybrid or BulkInsertion (the pasted section is large)
    assert!(
        profile.construction_pattern
            == provenance::forensics::docx::profile::ConstructionPattern::Hybrid
            || profile.construction_pattern
                == provenance::forensics::docx::profile::ConstructionPattern::BulkInsertion,
        "Should classify as Hybrid or BulkInsertion, got {:?}",
        profile.construction_pattern
    );

    // Should have paste-related anomalies
    assert!(
        !profile.anomalies.is_empty(),
        "Should detect anomalies in hybrid document"
    );

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// 12.7: Edge case tests
// ============================================================================

#[test]
fn test_docx_edge_case_no_metadata() {
    let path = temp_docx("no_metadata.docx");
    let mut builder = DocxBuilder::new();

    // No core or app metadata
    builder.add_paragraph("00AA1111", "Just a paragraph.");

    builder.write_to_file(&path).unwrap();

    // Should not panic, should handle gracefully
    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    assert!(profile.metadata.author.is_none());
    assert!(profile.metadata.total_editing_time_minutes.is_none());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_docx_edge_case_short_document() {
    let path = temp_docx("short_doc.docx");
    let mut builder = DocxBuilder::new();

    builder
        .set_core_metadata("Writer", "2024-01-01T10:00:00Z", "2024-01-01T10:30:00Z", 3)
        .add_paragraph("AA11", "Short.")
        .add_paragraph("BB22", "Very short.");

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();

    // With only 2 paragraphs, classification should be Insufficient
    assert_eq!(
        profile.construction_pattern,
        provenance::forensics::docx::profile::ConstructionPattern::Insufficient,
        "Short document should be Insufficient, got {:?}",
        profile.construction_pattern
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_docx_edge_case_google_export() {
    let path = temp_docx("google_export.docx");
    let mut builder = DocxBuilder::new();

    // Google Docs exports typically have minimal metadata
    builder.set_core_metadata("User", "2024-01-01T00:00:00Z", "2024-06-15T00:00:00Z", 1);
    builder.set_app_metadata("Google Docs", 0, 2000, 8);

    // Google Docs exports may have all paragraphs with the same rsid
    for i in 0..15 {
        builder.add_paragraph("00000000", &format!("Google Docs paragraph {i}."));
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();

    // Should handle gracefully — not crash, may classify as BulkInsertion or Insufficient
    assert!(
        profile.construction_pattern
            != provenance::forensics::docx::profile::ConstructionPattern::Organic,
        "Google export with uniform RSIDs should not be classified as Organic"
    );

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// 12.7: Output format tests
// ============================================================================

#[test]
fn test_docx_forensics_text_output() {
    let path = temp_docx("output_text.docx");
    let mut builder = DocxBuilder::new();
    builder
        .set_core_metadata("Author", "2024-01-01T10:00:00Z", "2024-01-15T16:00:00Z", 10)
        .set_app_metadata("Microsoft Office Word", 300, 2000, 8)
        .add_paragraph("AA11", "Test paragraph.");

    builder.write_to_file(&path).unwrap();

    let output = provenance::run_docx_forensics(
        path.to_str().unwrap(),
        provenance::scoring::engine::OutputFormat::Text,
    )
    .unwrap();

    assert!(output.contains("Document Construction Profile"));
    assert!(output.contains("RSID Analysis"));
    assert!(output.contains("Assessment"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_docx_forensics_json_output() {
    let path = temp_docx("output_json.docx");
    let mut builder = DocxBuilder::new();
    builder
        .set_core_metadata("Author", "2024-01-01T10:00:00Z", "2024-01-15T16:00:00Z", 10)
        .add_paragraph("AA11", "Test paragraph.");

    builder.write_to_file(&path).unwrap();

    let output = provenance::run_docx_forensics(
        path.to_str().unwrap(),
        provenance::scoring::engine::OutputFormat::Json,
    )
    .unwrap();

    // Should parse as valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("metadata").is_some());
    assert!(parsed.get("rsid_analysis").is_some());
    assert!(parsed.get("construction_pattern").is_some());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_docx_forensics_html_output() {
    let path = temp_docx("output_html.docx");
    let mut builder = DocxBuilder::new();
    builder
        .set_core_metadata("Author", "2024-01-01T10:00:00Z", "2024-01-15T16:00:00Z", 10)
        .add_paragraph("AA11", "Test paragraph.");

    builder.write_to_file(&path).unwrap();

    let output = provenance::run_docx_forensics(
        path.to_str().unwrap(),
        provenance::scoring::engine::OutputFormat::Html,
    )
    .unwrap();

    assert!(output.contains("<!DOCTYPE html>"));
    assert!(output.contains("Document Construction Profile"));

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// Additional RSID analysis tests
// ============================================================================

#[test]
fn test_rsid_revision_scatter_high() {
    let path = temp_docx("rsid_scatter_high.docx");
    let mut builder = DocxBuilder::new();

    // Alternating RSIDs — high revision scatter (writer revisited paragraphs)
    for i in 0..12 {
        let rsid = if i % 2 == 0 { "AAAA" } else { "BBBB" };
        builder.add_paragraph(rsid, &format!("Revised paragraph {i}."));
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    assert!(
        profile.rsid_analysis.revision_scatter > 0.5,
        "Alternating RSIDs should have high scatter, got {}",
        profile.rsid_analysis.revision_scatter
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_rsid_revision_scatter_low_with_diversity() {
    let path = temp_docx("rsid_scatter_low_diverse.docx");
    let mut builder = DocxBuilder::new();

    // Sequential blocks — low scatter but high diversity (long sessions)
    for i in 0..4 {
        let rsid = format!("{:08X}", i * 0x10000);
        for j in 0..5 {
            builder.add_paragraph(&rsid, &format!("Session {i} paragraph {j}."));
        }
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    // Scatter should be relatively low (only changes at block boundaries)
    assert!(
        profile.rsid_analysis.revision_scatter < 0.5,
        "Sequential blocks should have lower scatter, got {}",
        profile.rsid_analysis.revision_scatter
    );
    // But diversity should be moderate
    assert!(profile.rsid_analysis.unique_rsid_count == 4);

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// Template-based document test
// ============================================================================

#[test]
fn test_docx_edge_case_template() {
    let path = temp_docx("template_based.docx");
    let mut builder = DocxBuilder::new();

    builder.set_core_metadata(
        "Template User",
        "2024-01-01T10:00:00Z",
        "2024-02-01T14:00:00Z",
        20,
    );
    builder.set_app_metadata("Microsoft Office Word", 600, 3000, 12);

    // Template rsid for boilerplate + author's own rsids
    for _ in 0..3 {
        builder.add_paragraph("TEMPLATE1", "This is template boilerplate text.");
    }
    for i in 0..12 {
        builder.add_paragraph(
            &format!("{:08X}", 0x00200000 + i * 0x00030000),
            &format!("Author's original paragraph {i}."),
        );
    }
    for _ in 0..2 {
        builder.add_paragraph("TEMPLATE1", "More template footer text.");
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    // Should recognize the mix — not classify as pure BulkInsertion
    assert!(
        profile.construction_pattern
            != provenance::forensics::docx::profile::ConstructionPattern::BulkInsertion,
        "Template-based document should not be pure BulkInsertion"
    );

    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// Assessment text quality tests
// ============================================================================

#[test]
fn test_assessment_text_never_says_ai() {
    let path = temp_docx("assessment_lang.docx");
    let mut builder = DocxBuilder::new();

    builder.set_core_metadata("Writer", "2024-01-01T10:00:00Z", "2024-01-01T10:05:00Z", 1);
    builder.set_app_metadata("Microsoft Office Word", 5, 4000, 12);
    for i in 0..20 {
        builder.add_paragraph("SINGLE", &format!("Paragraph {i}."));
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();
    let text = profile.assessment_text.to_lowercase();

    // Assessment should NEVER say "AI-generated", "written by AI", etc.
    assert!(
        !text.contains("ai-generated"),
        "Assessment should never say 'AI-generated'"
    );
    assert!(
        !text.contains("written by ai"),
        "Assessment should never say 'written by AI'"
    );
    assert!(
        !text.contains("not human"),
        "Assessment should never say 'not human'"
    );
    assert!(
        !text.contains("cheating"),
        "Assessment should never say 'cheating'"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_anomaly_descriptions_use_question_framing() {
    let path = temp_docx("anomaly_framing.docx");
    let mut builder = DocxBuilder::new();

    builder.set_core_metadata("Writer", "2024-01-01T10:00:00Z", "2024-01-01T10:02:00Z", 1);
    builder.set_app_metadata("Microsoft Office Word", 2, 5000, 15);
    for i in 0..30 {
        builder.add_paragraph("BULK", &format!("Paragraph {i}."));
    }

    builder.write_to_file(&path).unwrap();

    let profile = provenance::forensics::docx::analyze_docx(&path).unwrap();

    for anomaly in &profile.anomalies {
        let desc = anomaly.description.to_lowercase();
        // Anomaly descriptions should never be accusatory
        assert!(
            !desc.contains("is ai"),
            "Anomaly description should not accuse: '{}'",
            anomaly.description
        );
        assert!(
            !desc.contains("was generated"),
            "Anomaly description should not accuse: '{}'",
            anomaly.description
        );
    }

    let _ = std::fs::remove_file(&path);
}
