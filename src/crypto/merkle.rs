//! Merkle tree for tamper-evident hash chains in Provenance certificates.
//!
//! Each leaf is a SHA-256 hash of a certificate field (document hash, analysis summary,
//! ACS score, etc.). The Merkle root binds all fields together so that any modification
//! to any field invalidates the certificate signature.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A Merkle tree built from labeled leaf entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// The root hash of the tree.
    pub root: String,
    /// Leaf entries (label + hash).
    pub leaves: Vec<MerkleLeaf>,
    /// Internal node hashes (bottom-up, left-to-right). Includes the root.
    pub nodes: Vec<String>,
}

/// A labeled leaf in the Merkle tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleLeaf {
    /// Human-readable label for the field.
    pub label: String,
    /// SHA-256 hash of the field value.
    pub hash: String,
}

/// An inclusion proof for a single leaf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Index of the leaf being proved.
    pub leaf_index: usize,
    /// The leaf's label and hash.
    pub leaf: MerkleLeaf,
    /// Sibling hashes needed to reconstruct the root (from leaf to root).
    /// Each entry is (hash, is_right_sibling).
    pub siblings: Vec<(String, bool)>,
    /// Expected root hash.
    pub root: String,
}

/// Build a Merkle tree from labeled data fields.
///
/// Each entry is (label, data_bytes). The leaf hash is SHA-256(label || ":" || data).
pub fn build(entries: &[(&str, &[u8])]) -> MerkleTree {
    if entries.is_empty() {
        return MerkleTree {
            root: hash_bytes(b"empty"),
            leaves: Vec::new(),
            nodes: vec![hash_bytes(b"empty")],
        };
    }

    // Build leaves
    let leaves: Vec<MerkleLeaf> = entries
        .iter()
        .map(|(label, data)| {
            let mut hasher = Sha256::new();
            hasher.update(label.as_bytes());
            hasher.update(b":");
            hasher.update(data);
            let hash = format!("{:x}", hasher.finalize());
            MerkleLeaf {
                label: label.to_string(),
                hash,
            }
        })
        .collect();

    // Collect leaf hashes as the initial layer
    let mut current_layer: Vec<String> = leaves.iter().map(|l| l.hash.clone()).collect();
    let mut all_nodes: Vec<String> = current_layer.clone();

    // Build tree bottom-up
    while current_layer.len() > 1 {
        let mut next_layer = Vec::new();
        let mut i = 0;
        while i < current_layer.len() {
            if i + 1 < current_layer.len() {
                let combined = hash_pair(&current_layer[i], &current_layer[i + 1]);
                next_layer.push(combined);
            } else {
                // Odd node: promote as-is
                next_layer.push(current_layer[i].clone());
            }
            i += 2;
        }
        all_nodes.extend(next_layer.clone());
        current_layer = next_layer;
    }

    let root = current_layer.into_iter().next().unwrap_or_else(|| hash_bytes(b"empty"));

    MerkleTree {
        root,
        leaves,
        nodes: all_nodes,
    }
}

/// Generate an inclusion proof for a specific leaf index.
pub fn prove(tree: &MerkleTree, leaf_index: usize) -> Option<MerkleProof> {
    if leaf_index >= tree.leaves.len() {
        return None;
    }

    let leaf_hashes: Vec<&str> = tree.leaves.iter().map(|l| l.hash.as_str()).collect();
    let mut siblings = Vec::new();
    let mut current_layer: Vec<String> = leaf_hashes.iter().map(|s| s.to_string()).collect();
    let mut idx = leaf_index;

    while current_layer.len() > 1 {
        let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };

        if sibling_idx < current_layer.len() {
            let is_right = idx % 2 == 0;
            siblings.push((current_layer[sibling_idx].clone(), is_right));
        }

        // Build next layer
        let mut next_layer = Vec::new();
        let mut i = 0;
        while i < current_layer.len() {
            if i + 1 < current_layer.len() {
                next_layer.push(hash_pair(&current_layer[i], &current_layer[i + 1]));
            } else {
                next_layer.push(current_layer[i].clone());
            }
            i += 2;
        }
        current_layer = next_layer;
        idx /= 2;
    }

    Some(MerkleProof {
        leaf_index,
        leaf: tree.leaves[leaf_index].clone(),
        siblings,
        root: tree.root.clone(),
    })
}

/// Verify a Merkle inclusion proof.
pub fn verify_proof(proof: &MerkleProof) -> bool {
    let mut current = proof.leaf.hash.clone();

    for (sibling, is_right_sibling) in &proof.siblings {
        current = if *is_right_sibling {
            hash_pair(&current, sibling)
        } else {
            hash_pair(sibling, &current)
        };
    }

    current == proof.root
}

/// Hash two child hashes to form a parent.
fn hash_pair(left: &str, right: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Hash raw bytes.
fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_single_leaf() {
        let tree = build(&[("doc_hash", b"abc123")]);
        assert_eq!(tree.leaves.len(), 1);
        assert!(!tree.root.is_empty());
        assert_eq!(tree.root, tree.leaves[0].hash);
    }

    #[test]
    fn test_build_multiple_leaves() {
        let tree = build(&[
            ("doc_hash", b"abc123"),
            ("acs_score", b"75.0"),
            ("pii_score", b"82.0"),
            ("timestamp", b"2026-03-22T00:00:00Z"),
        ]);
        assert_eq!(tree.leaves.len(), 4);
        // Root should differ from any individual leaf
        for leaf in &tree.leaves {
            assert_ne!(tree.root, leaf.hash);
        }
    }

    #[test]
    fn test_build_empty() {
        let tree = build(&[]);
        assert!(tree.leaves.is_empty());
        assert!(!tree.root.is_empty());
    }

    #[test]
    fn test_merkle_proof_valid() {
        let tree = build(&[
            ("field_a", b"value_a"),
            ("field_b", b"value_b"),
            ("field_c", b"value_c"),
        ]);

        for i in 0..tree.leaves.len() {
            let proof = prove(&tree, i).unwrap();
            assert!(verify_proof(&proof), "Proof for leaf {i} should be valid");
        }
    }

    #[test]
    fn test_merkle_proof_tampered() {
        let tree = build(&[
            ("field_a", b"value_a"),
            ("field_b", b"value_b"),
        ]);

        let mut proof = prove(&tree, 0).unwrap();
        // Tamper with the leaf hash
        proof.leaf.hash = "0000000000000000000000000000000000000000000000000000000000000000".into();
        assert!(!verify_proof(&proof), "Tampered proof should fail");
    }

    #[test]
    fn test_deterministic() {
        let entries: &[(&str, &[u8])] = &[("a", b"1"), ("b", b"2")];
        let tree1 = build(entries);
        let tree2 = build(entries);
        assert_eq!(tree1.root, tree2.root);
    }
}
