//! Provenance Certificate — tamper-evident attestation of analysis results.
//!
//! A certificate bundles the document hash, analysis scores, metadata, and timestamps
//! into a Merkle tree, then signs the root with an Ed25519 key. This allows anyone
//! with the public key to verify that no field has been modified after issuance.

use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};

use super::keys::PublicKeyInfo;
use super::merkle::{self, MerkleTree};
use crate::utils::errors::Result;

/// A tamper-evident Provenance certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceCertificate {
    /// Certificate format version.
    pub version: u32,
    /// Unique certificate ID (hex-encoded random bytes).
    pub certificate_id: String,
    /// ISO 8601 timestamp of certificate generation.
    pub issued_at: String,
    /// Software version that generated this certificate.
    pub software_version: String,

    /// Document being attested.
    pub document: DocumentAttestation,
    /// Analysis results summary.
    pub analysis_summary: AnalysisSummary,
    /// Process evidence summary.
    pub process_summary: ProcessSummary,

    /// Merkle tree binding all fields.
    pub merkle_tree: MerkleTree,

    /// Ed25519 signature over the Merkle root.
    pub signature: String,
    /// Public key that can verify the signature.
    pub signing_key: PublicKeyInfo,
}

/// Attestation about the analyzed document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAttestation {
    /// Original file name.
    pub file_name: String,
    /// SHA-256 of the original file.
    pub file_hash: String,
    /// File size in bytes.
    pub file_size: u64,
    /// Detected format.
    pub format: String,
    /// Word count of extracted text.
    pub word_count: usize,
}

/// Summary of analysis scores included in the certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    /// Authorship Confidence Score (0-100).
    pub acs_score: f64,
    /// ACS confidence interval margin.
    pub acs_margin: f64,
    /// Process Integrity Index (0-100).
    pub pii_score: f64,
    /// PII level label.
    pub pii_level: String,
    /// Register classification.
    pub register: String,
    /// Number of anomaly flags.
    pub anomaly_count: usize,
    /// Construction pattern (for DOCX files).
    pub construction_pattern: Option<String>,
}

/// Summary of process evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSummary {
    /// Whether DOCX forensics were available.
    pub has_docx_forensics: bool,
    /// Whether an author profile was compared.
    pub has_author_profile: bool,
    /// Number of RSID editing sessions (if DOCX).
    pub rsid_session_count: Option<usize>,
    /// Formatting consistency score (if DOCX).
    pub formatting_consistency: Option<f64>,
}

/// Input data needed to generate a certificate.
pub struct CertificateInput<'a> {
    pub file_name: &'a str,
    pub file_hash: &'a str,
    pub file_size: u64,
    pub format: &'a str,
    pub word_count: usize,
    pub acs_score: f64,
    pub acs_margin: f64,
    pub pii_score: f64,
    pub pii_level: &'a str,
    pub register: &'a str,
    pub anomaly_count: usize,
    pub construction_pattern: Option<&'a str>,
    pub has_docx_forensics: bool,
    pub has_author_profile: bool,
    pub rsid_session_count: Option<usize>,
    pub formatting_consistency: Option<f64>,
}

/// Generate a signed Provenance certificate.
pub fn generate(input: &CertificateInput, signing_key: &SigningKey) -> Result<ProvenanceCertificate> {
    let issued_at = current_iso8601();
    let certificate_id = generate_certificate_id();

    let document = DocumentAttestation {
        file_name: input.file_name.to_string(),
        file_hash: input.file_hash.to_string(),
        file_size: input.file_size,
        format: input.format.to_string(),
        word_count: input.word_count,
    };

    let analysis_summary = AnalysisSummary {
        acs_score: input.acs_score,
        acs_margin: input.acs_margin,
        pii_score: input.pii_score,
        pii_level: input.pii_level.to_string(),
        register: input.register.to_string(),
        anomaly_count: input.anomaly_count,
        construction_pattern: input.construction_pattern.map(|s| s.to_string()),
    };

    let process_summary = ProcessSummary {
        has_docx_forensics: input.has_docx_forensics,
        has_author_profile: input.has_author_profile,
        rsid_session_count: input.rsid_session_count,
        formatting_consistency: input.formatting_consistency,
    };

    // Build Merkle tree from all certificate fields
    let merkle_entries = build_merkle_entries(
        &certificate_id,
        &issued_at,
        &document,
        &analysis_summary,
        &process_summary,
    );
    let entry_refs: Vec<(&str, &[u8])> = merkle_entries
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_bytes()))
        .collect();
    let merkle_tree = merkle::build(&entry_refs);

    // Sign the Merkle root
    use base64::Engine;
    let signature_bytes = signing_key.sign(merkle_tree.root.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(signature_bytes.to_bytes());

    let signing_key_info = PublicKeyInfo::from_verifying_key(&signing_key.verifying_key());

    Ok(ProvenanceCertificate {
        version: 1,
        certificate_id,
        issued_at,
        software_version: env!("CARGO_PKG_VERSION").to_string(),
        document,
        analysis_summary,
        process_summary,
        merkle_tree,
        signature,
        signing_key: signing_key_info,
    })
}

/// Serialize a certificate to JSON.
pub fn to_json(cert: &ProvenanceCertificate) -> Result<String> {
    Ok(serde_json::to_string_pretty(cert)?)
}

/// Deserialize a certificate from JSON.
pub fn from_json(json: &str) -> Result<ProvenanceCertificate> {
    Ok(serde_json::from_str(json)?)
}

/// Save a certificate to a JSON file.
pub fn save(cert: &ProvenanceCertificate, path: &std::path::Path) -> Result<()> {
    let json = to_json(cert)?;
    std::fs::write(path, json).map_err(|e| crate::utils::errors::ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })
}

/// Load a certificate from a JSON file.
pub fn load(path: &std::path::Path) -> Result<ProvenanceCertificate> {
    let content = crate::utils::errors::read_file_string(path)?;
    from_json(&content)
}

/// Build labeled entries for the Merkle tree.
fn build_merkle_entries(
    certificate_id: &str,
    issued_at: &str,
    doc: &DocumentAttestation,
    analysis: &AnalysisSummary,
    process: &ProcessSummary,
) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    // Certificate metadata
    entries.push(("certificate_id".into(), certificate_id.to_string()));
    entries.push(("issued_at".into(), issued_at.to_string()));
    entries.push(("software_version".into(), env!("CARGO_PKG_VERSION").to_string()));

    // Document fields
    entries.push(("doc.file_name".into(), doc.file_name.clone()));
    entries.push(("doc.file_hash".into(), doc.file_hash.clone()));
    entries.push(("doc.file_size".into(), doc.file_size.to_string()));
    entries.push(("doc.format".into(), doc.format.clone()));
    entries.push(("doc.word_count".into(), doc.word_count.to_string()));

    // Analysis fields
    entries.push(("analysis.acs_score".into(), format!("{:.2}", analysis.acs_score)));
    entries.push(("analysis.acs_margin".into(), format!("{:.2}", analysis.acs_margin)));
    entries.push(("analysis.pii_score".into(), format!("{:.2}", analysis.pii_score)));
    entries.push(("analysis.pii_level".into(), analysis.pii_level.clone()));
    entries.push(("analysis.register".into(), analysis.register.clone()));
    entries.push(("analysis.anomaly_count".into(), analysis.anomaly_count.to_string()));
    if let Some(ref cp) = analysis.construction_pattern {
        entries.push(("analysis.construction_pattern".into(), cp.clone()));
    }

    // Process evidence
    entries.push(("process.has_docx_forensics".into(), process.has_docx_forensics.to_string()));
    entries.push(("process.has_author_profile".into(), process.has_author_profile.to_string()));
    if let Some(rsid) = process.rsid_session_count {
        entries.push(("process.rsid_sessions".into(), rsid.to_string()));
    }
    if let Some(fmt) = process.formatting_consistency {
        entries.push(("process.formatting_consistency".into(), format!("{:.3}", fmt)));
    }

    entries
}

fn current_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    crate::scoring::engine::format_epoch_public(secs)
}

fn generate_certificate_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys;

    fn sample_input() -> CertificateInput<'static> {
        CertificateInput {
            file_name: "essay.docx",
            file_hash: "abc123def456",
            file_size: 45000,
            format: "docx",
            word_count: 3200,
            acs_score: 72.5,
            acs_margin: 12.0,
            pii_score: 65.0,
            pii_level: "Moderate",
            register: "Academic",
            anomaly_count: 2,
            construction_pattern: Some("Organic"),
            has_docx_forensics: true,
            has_author_profile: false,
            rsid_session_count: Some(14),
            formatting_consistency: Some(0.92),
        }
    }

    #[test]
    fn test_generate_certificate() {
        let key = keys::generate_keypair();
        let input = sample_input();
        let cert = generate(&input, &key).unwrap();

        assert_eq!(cert.version, 1);
        assert_eq!(cert.document.file_name, "essay.docx");
        assert_eq!(cert.analysis_summary.acs_score, 72.5);
        assert!(!cert.signature.is_empty());
        assert!(!cert.merkle_tree.root.is_empty());
        assert_eq!(cert.certificate_id.len(), 32); // 16 bytes hex
    }

    #[test]
    fn test_certificate_json_roundtrip() {
        let key = keys::generate_keypair();
        let input = sample_input();
        let cert = generate(&input, &key).unwrap();

        let json = to_json(&cert).unwrap();
        let recovered = from_json(&json).unwrap();

        assert_eq!(cert.certificate_id, recovered.certificate_id);
        assert_eq!(cert.merkle_tree.root, recovered.merkle_tree.root);
        assert_eq!(cert.signature, recovered.signature);
    }

    #[test]
    fn test_certificate_save_load() {
        let key = keys::generate_keypair();
        let input = sample_input();
        let cert = generate(&input, &key).unwrap();

        let dir = std::env::temp_dir().join("provenance_test_cert");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_cert.json");

        save(&cert, &path).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(cert.certificate_id, loaded.certificate_id);
        assert_eq!(cert.signature, loaded.signature);

        std::fs::remove_dir_all(&dir).ok();
    }
}
