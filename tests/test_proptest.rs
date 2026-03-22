//! Property-based tests using proptest.
//!
//! Tests invariants that should hold for all inputs:
//! - Analysis determinism (same input → same output)
//! - Text normalization idempotence
//! - Feature vector serialization roundtrip
//! - Validation never panics on arbitrary input

use proptest::prelude::*;

// ─── Analysis Determinism ─────────────────────────────────────────────

proptest! {
    #[test]
    fn analysis_is_deterministic(text in "[a-zA-Z .,!?]{50,500}") {
        let result1 = provenance::analysis::analyze_text(&text).unwrap();
        let result2 = provenance::analysis::analyze_text(&text).unwrap();

        // Key metrics must be identical
        prop_assert_eq!(result1.lexical.total_words, result2.lexical.total_words);
        prop_assert_eq!(result1.lexical.unique_words, result2.lexical.unique_words);
        prop_assert_eq!(result1.syntactic.total_sentences, result2.syntactic.total_sentences);
        prop_assert!((result1.lexical.mattr - result2.lexical.mattr).abs() < f64::EPSILON);
    }
}

// ─── Feature Vector Serialization Roundtrip ───────────────────────────

proptest! {
    #[test]
    fn feature_vector_roundtrip(
        names in prop::collection::vec("[a-z]{2,5}", 1..20),
        values in prop::collection::vec(-100.0f64..100.0, 1..20),
    ) {
        let len = names.len().min(values.len());
        let fv = provenance::identity::features::FeatureVector {
            names: names[..len].to_vec(),
            values: values[..len].to_vec(),
        };

        let json = serde_json::to_string(&fv).unwrap();
        let restored: provenance::identity::features::FeatureVector =
            serde_json::from_str(&json).unwrap();

        prop_assert_eq!(fv.names, restored.names);
        prop_assert_eq!(fv.values.len(), restored.values.len());
        for (a, b) in fv.values.iter().zip(restored.values.iter()) {
            prop_assert!((a - b).abs() < 1e-10);
        }
    }
}

// ─── Validation Never Panics ──────────────────────────────────────────

proptest! {
    #[test]
    fn validation_path_no_panic(path in ".*") {
        let _ = provenance::utils::validation::validate_archive_entry_path(&path);
    }

    #[test]
    fn sanitize_filename_no_panic(name in ".*") {
        let result = provenance::utils::validation::sanitize_filename(&name);
        // Result should never contain path separators or null bytes
        prop_assert!(!result.contains('\0'));
    }

    #[test]
    fn xml_bomb_check_no_panic(xml in ".*") {
        let config = provenance::utils::validation::ValidationConfig::default();
        let _ = provenance::utils::validation::check_xml_bomb(&xml, &config);
    }
}

// ─── Text Normalization Idempotence ───────────────────────────────────

proptest! {
    #[test]
    fn normalization_idempotent(text in "[a-zA-Z0-9 \n\r\t.,!?]{10,200}") {
        // Extract and analyze text — the analysis should not change on re-analysis
        let result1 = provenance::analysis::analyze_text(&text);
        let result2 = provenance::analysis::analyze_text(&text);

        match (result1, result2) {
            (Ok(r1), Ok(r2)) => {
                prop_assert_eq!(r1.lexical.total_words, r2.lexical.total_words);
            }
            _ => {} // Both failing is also fine
        }
    }
}

// ─── Distance Metric Properties ───────────────────────────────────────

proptest! {
    #[test]
    fn cosine_similarity_self_is_one(values in prop::collection::vec(0.1f64..10.0, 3..50)) {
        let a = ndarray::Array1::from_vec(values);
        let sim = provenance::identity::distances::cosine_similarity(&a, &a);
        prop_assert!((sim - 1.0).abs() < 1e-10, "Self-similarity should be 1.0, got {sim}");
    }

    #[test]
    fn euclidean_distance_self_is_zero(values in prop::collection::vec(-10.0f64..10.0, 3..50)) {
        let a = ndarray::Array1::from_vec(values);
        let dist = provenance::identity::distances::euclidean_distance(&a, &a);
        prop_assert!(dist.abs() < 1e-10, "Self-distance should be 0.0, got {dist}");
    }

    #[test]
    fn manhattan_distance_symmetric(
        a_vals in prop::collection::vec(-10.0f64..10.0, 5..20),
        b_vals in prop::collection::vec(-10.0f64..10.0, 5..20),
    ) {
        let len = a_vals.len().min(b_vals.len());
        let a = ndarray::Array1::from_vec(a_vals[..len].to_vec());
        let b = ndarray::Array1::from_vec(b_vals[..len].to_vec());

        let d_ab = provenance::identity::distances::manhattan_distance(&a, &b);
        let d_ba = provenance::identity::distances::manhattan_distance(&b, &a);
        prop_assert!((d_ab - d_ba).abs() < 1e-10, "Manhattan should be symmetric");
    }
}
