"""MinHash-based deduplication for the training data pipeline.

Uses datasketch for efficient approximate duplicate detection across
large document collections. Configurable similarity threshold and
batch processing for memory efficiency.

Usage:
    from dedup import MinHashDeduplicator
    dedup = MinHashDeduplicator(threshold=0.8)
    unique_docs = dedup.deduplicate(documents)
"""

import hashlib
import logging
from typing import Iterator

from datasketch import MinHash, MinHashLSH

logger = logging.getLogger(__name__)

# Number of permutations for MinHash (higher = more accurate, slower)
NUM_PERM = 128

# Number of shingles (word n-grams) for MinHash
SHINGLE_SIZE = 5


class MinHashDeduplicator:
    """Approximate near-duplicate detection using MinHash LSH.

    Args:
        threshold: Jaccard similarity threshold for considering documents
            as duplicates. Default 0.8.
        num_perm: Number of MinHash permutations. Default 128.
        shingle_size: Word n-gram size for shingling. Default 5.
    """

    def __init__(
        self,
        threshold: float = 0.8,
        num_perm: int = NUM_PERM,
        shingle_size: int = SHINGLE_SIZE,
    ):
        self.threshold = threshold
        self.num_perm = num_perm
        self.shingle_size = shingle_size
        self.lsh = MinHashLSH(threshold=threshold, num_perm=num_perm)
        self._seen_keys: set[str] = set()

    def _text_to_shingles(self, text: str) -> set[str]:
        """Convert text to a set of word-level shingles."""
        words = text.lower().split()
        if len(words) < self.shingle_size:
            return {" ".join(words)}
        return {
            " ".join(words[i : i + self.shingle_size])
            for i in range(len(words) - self.shingle_size + 1)
        }

    def _compute_minhash(self, text: str) -> MinHash:
        """Compute MinHash signature for a document."""
        m = MinHash(num_perm=self.num_perm)
        for shingle in self._text_to_shingles(text):
            m.update(shingle.encode("utf-8"))
        return m

    def _doc_key(self, text: str) -> str:
        """Generate a unique key for a document."""
        return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]

    def is_duplicate(self, text: str) -> bool:
        """Check if a document is a near-duplicate of any previously seen document.

        Also inserts the document into the index if it is not a duplicate.

        Returns:
            True if the document is a duplicate, False if it is unique.
        """
        key = self._doc_key(text)

        # Exact duplicate check
        if key in self._seen_keys:
            return True

        minhash = self._compute_minhash(text)

        # Query LSH for near-duplicates
        result = self.lsh.query(minhash)
        if result:
            return True

        # Not a duplicate — insert into index
        try:
            self.lsh.insert(key, minhash)
            self._seen_keys.add(key)
        except ValueError:
            # Key collision (extremely rare with sha256 prefix)
            pass

        return False

    def deduplicate(self, documents: list[dict]) -> list[dict]:
        """Remove near-duplicates from a list of documents.

        Args:
            documents: List of dicts, each must have a 'text' field.

        Returns:
            List of unique documents (order preserved).
        """
        unique = []
        duplicates_found = 0

        for doc in documents:
            text = doc.get("text", "")
            if not text or len(text.split()) < 50:
                # Skip very short documents
                continue

            if self.is_duplicate(text):
                duplicates_found += 1
            else:
                unique.append(doc)

        logger.info(
            "Deduplication: %d input → %d unique (%d duplicates removed)",
            len(documents),
            len(unique),
            duplicates_found,
        )
        return unique

    def deduplicate_stream(self, documents: Iterator[dict]) -> Iterator[dict]:
        """Streaming deduplication for memory efficiency.

        Yields unique documents one at a time without loading
        the full collection into memory.
        """
        total = 0
        duplicates = 0

        for doc in documents:
            total += 1
            text = doc.get("text", "")
            if not text or len(text.split()) < 50:
                continue

            if self.is_duplicate(text):
                duplicates += 1
            else:
                yield doc

            if total % 10_000 == 0:
                logger.info(
                    "Processed %d docs, %d duplicates found so far",
                    total,
                    duplicates,
                )

        logger.info(
            "Stream dedup complete: %d total, %d duplicates removed",
            total,
            duplicates,
        )

    @property
    def index_size(self) -> int:
        """Number of unique documents in the index."""
        return len(self._seen_keys)

    def reset(self):
        """Clear the deduplication index."""
        self.lsh = MinHashLSH(threshold=self.threshold, num_perm=self.num_perm)
        self._seen_keys.clear()
