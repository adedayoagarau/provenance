# Provenance Data Pipeline

## Corpus Format

Provenance expects corpora in a simple directory structure:

```
corpus_dir/
    author_alice/
        essay1.txt
        essay2.txt
        blog_post.md
    author_bob/
        article1.txt
        story1.txt
```

Each subdirectory name becomes the author label. All text file formats
supported by Provenance (TXT, MD, HTML, PDF, DOCX, RTF, ODT, EPUB, etc.)
can be used as input.

## Dataset Collection Scripts

### PAN Competition Datasets

```bash
# Download PAN data from https://pan.webis.de/ first
python data/scripts/download_pan.py pan --year 2020 --input pan2020_raw/ --output data/pan2020/

# List available datasets
python data/scripts/download_pan.py info
```

### Project Gutenberg

```bash
# Download .txt files from https://www.gutenberg.org/
# Name files as "Author Name - Title.txt"
python data/scripts/download_pan.py gutenberg --input gutenberg_raw/ --output data/gutenberg/
```

## Quality Criteria

The corpus loader applies these filters (configurable):

| Criterion | Default | Purpose |
|-----------|---------|---------|
| `min_words` | 100 | Skip fragments too short for stylometric analysis |
| `min_quality` | 0.3 | Skip documents with encoding issues or no sentences |
| `deduplicate` | true | Remove exact-duplicate texts by SHA-256 hash |

## Built-in Benchmark

A small benchmark corpus (5 authors, 10 texts each) is included in
`tests/fixtures/benchmark/` for automated testing. Run it with:

```bash
cargo test test_benchmark
```
