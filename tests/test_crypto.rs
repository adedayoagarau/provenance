use provenance::crypto::hashing;

#[test]
fn test_sha256_known_value() {
    let hash = hashing::sha256(b"hello world");
    assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
}

#[test]
fn test_sha256_empty() {
    let hash = hashing::sha256(b"");
    assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}
