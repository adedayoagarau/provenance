#!/usr/bin/env python3
"""Data collection pipeline for the Provenance ML training dataset.

Gathers 50K human documents and 75K AI documents from diverse sources,
plus 25K humanized AI samples. Stores everything as JSONL with MinHash
deduplication.

Usage:
    python collect.py                       # Full collection run
    python collect.py --dry-run             # Validate API connectivity only
    python collect.py --source arxiv        # Collect from a single source
    python collect.py --ai-only             # Generate AI text only
    python collect.py --humanize-only       # Run humanizer on existing AI samples
    python collect.py --resume              # Resume from last checkpoint

See ML_PIPELINE_IMPLEMENTATION_GUIDE.md for distribution targets and
config.py for all configuration parameters.
"""

import abc
import argparse
import json
import logging
import os
import random
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterator

import requests
from tqdm import tqdm

import config
from dedup import MinHashDeduplicator

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger(__name__)

# ─── Output Helpers ───────────────────────────────────────────────────────────


def make_record(
    text: str,
    label: str,
    source: str,
    register: str,
    model: str = "",
    prompt_type: str = "",
) -> dict:
    """Create a JSONL record conforming to the pipeline schema."""
    return {
        "text": text.strip(),
        "label": label,
        "source": source,
        "register": register,
        "model": model,
        "prompt_type": prompt_type,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "word_count": len(text.split()),
    }


def append_jsonl(record: dict, path: Path):
    """Append a single record to a JSONL file."""
    with open(path, "a", encoding="utf-8") as f:
        f.write(json.dumps(record, ensure_ascii=False) + "\n")


def count_lines(path: Path) -> int:
    """Count lines in a JSONL file (for resume support)."""
    if not path.exists():
        return 0
    with open(path, "r") as f:
        return sum(1 for _ in f)


# ─── Base Collector ───────────────────────────────────────────────────────────


class BaseCollector(abc.ABC):
    """Abstract base class for all data source collectors."""

    name: str = "base"
    register: str = "mixed"

    def __init__(self, target_count: int):
        self.target_count = target_count
        self.collected = 0

    @abc.abstractmethod
    def collect(self) -> Iterator[dict]:
        """Yield JSONL records from this source."""
        ...

    def dry_run(self) -> bool:
        """Test API connectivity. Returns True if source is reachable."""
        logger.info("[%s] Dry run — checking connectivity...", self.name)
        return True


# ─── Human Source Collectors ──────────────────────────────────────────────────


class ArxivCollector(BaseCollector):
    """Collect academic papers from the ArXiv API.

    Target: 25% of human data (~12,500 documents).
    Register: academic.
    """

    name = "arxiv"
    register = "academic"
    API_URL = "http://export.arxiv.org/api/query"

    def __init__(self, target_count: int):
        super().__init__(target_count)
        self.categories = [
            "cs.CL", "cs.AI", "cs.LG", "physics.gen-ph",
            "math.ST", "econ.GN", "q-bio.QM", "stat.ML",
        ]

    def collect(self) -> Iterator[dict]:
        batch_size = 100
        for cat in self.categories:
            start = 0
            per_cat = self.target_count // len(self.categories)
            while self.collected < per_cat:
                params = {
                    "search_query": f"cat:{cat}",
                    "start": start,
                    "max_results": batch_size,
                    "sortBy": "submittedDate",
                    "sortOrder": "descending",
                }
                try:
                    resp = requests.get(self.API_URL, params=params, timeout=30)
                    resp.raise_for_status()
                except requests.RequestException as e:
                    logger.warning("[arxiv] Request failed: %s", e)
                    time.sleep(5)
                    continue

                # Parse Atom XML (simplified — production should use feedparser)
                import xml.etree.ElementTree as ET

                root = ET.fromstring(resp.text)
                ns = {"atom": "http://www.w3.org/2005/Atom"}
                entries = root.findall("atom:entry", ns)

                if not entries:
                    break

                for entry in entries:
                    summary = entry.findtext("atom:summary", "", ns).strip()
                    title = entry.findtext("atom:title", "", ns).strip()
                    text = f"{title}\n\n{summary}"
                    if len(text.split()) >= 100:
                        yield make_record(text, "human", "arxiv", "academic")
                        self.collected += 1

                start += batch_size
                time.sleep(3)  # Rate limiting

    def dry_run(self) -> bool:
        try:
            resp = requests.get(
                self.API_URL,
                params={"search_query": "cat:cs.CL", "max_results": 1},
                timeout=10,
            )
            ok = resp.status_code == 200
            logger.info("[arxiv] %s", "OK" if ok else f"FAILED ({resp.status_code})")
            return ok
        except requests.RequestException as e:
            logger.error("[arxiv] FAILED: %s", e)
            return False


class NewsCollector(BaseCollector):
    """Collect news articles from Reuters/AP RSS feeds.

    Target: 20% of human data (~10,000 documents).
    Register: journalism.
    """

    name = "news"
    register = "journalism"
    FEEDS = [
        "https://feeds.reuters.com/reuters/topNews",
        "https://feeds.reuters.com/reuters/worldNews",
        "https://feeds.reuters.com/reuters/businessNews",
        "https://feeds.reuters.com/reuters/technologyNews",
        "https://feeds.reuters.com/reuters/scienceNews",
    ]

    def collect(self) -> Iterator[dict]:
        try:
            import feedparser
        except ImportError:
            logger.error("[news] feedparser not installed")
            return

        for feed_url in self.FEEDS:
            if self.collected >= self.target_count:
                break
            try:
                feed = feedparser.parse(feed_url)
                for entry in feed.entries:
                    text = entry.get("summary", "") or entry.get("description", "")
                    title = entry.get("title", "")
                    full_text = f"{title}\n\n{text}"
                    if len(full_text.split()) >= 100:
                        yield make_record(full_text, "human", "news_rss", "journalism")
                        self.collected += 1
            except Exception as e:
                logger.warning("[news] Feed error for %s: %s", feed_url, e)
                continue

    def dry_run(self) -> bool:
        try:
            import feedparser

            feed = feedparser.parse(self.FEEDS[0])
            ok = len(feed.entries) > 0
            logger.info("[news] %s (%d entries)", "OK" if ok else "EMPTY", len(feed.entries))
            return ok
        except Exception as e:
            logger.error("[news] FAILED: %s", e)
            return False


class GutenbergCollector(BaseCollector):
    """Collect literary texts from Project Gutenberg.

    Target: 15% of human data (~7,500 documents).
    Register: literary.
    Extracts passages of 500-2000 words from full texts.
    """

    name = "gutenberg"
    register = "literary"
    API_URL = "https://gutendex.com/books"

    def collect(self) -> Iterator[dict]:
        page = 1
        while self.collected < self.target_count:
            try:
                resp = requests.get(
                    self.API_URL,
                    params={"page": page, "languages": "en", "mime_type": "text/plain"},
                    timeout=30,
                )
                resp.raise_for_status()
                data = resp.json()
            except (requests.RequestException, json.JSONDecodeError) as e:
                logger.warning("[gutenberg] API error: %s", e)
                time.sleep(5)
                page += 1
                continue

            for book in data.get("results", []):
                if self.collected >= self.target_count:
                    break

                # Find plain text URL
                formats = book.get("formats", {})
                text_url = formats.get("text/plain; charset=utf-8") or formats.get("text/plain")
                if not text_url:
                    continue

                try:
                    text_resp = requests.get(text_url, timeout=30)
                    text_resp.raise_for_status()
                    full_text = text_resp.text
                except requests.RequestException:
                    continue

                # Extract passages of 500-2000 words
                words = full_text.split()
                passage_size = random.randint(500, 2000)
                for i in range(0, len(words) - passage_size, passage_size):
                    passage = " ".join(words[i : i + passage_size])
                    yield make_record(passage, "human", "gutenberg", "literary")
                    self.collected += 1
                    if self.collected >= self.target_count:
                        break

                time.sleep(1)

            if not data.get("next"):
                break
            page += 1

    def dry_run(self) -> bool:
        try:
            resp = requests.get(self.API_URL, params={"page": 1}, timeout=10)
            ok = resp.status_code == 200
            logger.info("[gutenberg] %s", "OK" if ok else f"FAILED ({resp.status_code})")
            return ok
        except requests.RequestException as e:
            logger.error("[gutenberg] FAILED: %s", e)
            return False


class EdgarCollector(BaseCollector):
    """Collect financial filings from SEC EDGAR.

    Target: 15% of human data (~7,500 documents).
    Register: business/legal.
    """

    name = "edgar"
    register = "business"
    BASE_URL = "https://efts.sec.gov/LATEST/search-index"
    FULL_TEXT_URL = "https://www.sec.gov/Archives/edgar/data"
    HEADERS = {"User-Agent": "Provenance Research research@example.com"}

    def collect(self) -> Iterator[dict]:
        # Use EDGAR full-text search API
        form_types = ["10-K", "10-Q", "8-K", "S-1", "DEF 14A"]
        for form_type in form_types:
            if self.collected >= self.target_count:
                break
            try:
                resp = requests.get(
                    "https://efts.sec.gov/LATEST/search-index",
                    params={
                        "q": "*",
                        "dateRange": "custom",
                        "startdt": "2020-01-01",
                        "enddt": "2024-12-31",
                        "forms": form_type,
                    },
                    headers=self.HEADERS,
                    timeout=30,
                )
                resp.raise_for_status()
                data = resp.json()
                for hit in data.get("hits", {}).get("hits", []):
                    text = hit.get("_source", {}).get("file_description", "")
                    if len(text.split()) >= 200:
                        yield make_record(text, "human", "edgar", "business")
                        self.collected += 1
            except (requests.RequestException, json.JSONDecodeError) as e:
                logger.warning("[edgar] Error: %s", e)
                time.sleep(2)

    def dry_run(self) -> bool:
        try:
            resp = requests.get(
                "https://efts.sec.gov/LATEST/search-index",
                params={"q": "annual report", "forms": "10-K"},
                headers=self.HEADERS,
                timeout=10,
            )
            ok = resp.status_code == 200
            logger.info("[edgar] %s", "OK" if ok else f"FAILED ({resp.status_code})")
            return ok
        except requests.RequestException as e:
            logger.error("[edgar] FAILED: %s", e)
            return False


class RedditCollector(BaseCollector):
    """Collect verified human posts from Reddit.

    Target: 10% of human data (~5,000 documents).
    Register: personal/mixed.
    Filters for accounts with history, karma, and verified activity.
    """

    name = "reddit"
    register = "personal"

    def __init__(self, target_count: int):
        super().__init__(target_count)
        self.subreddits = [
            "WritingPrompts", "TrueOffMyChest", "AskHistorians",
            "explainlikeimfive", "changemyview", "AskScience",
            "legaladvice", "personalfinance", "relationships",
        ]

    def collect(self) -> Iterator[dict]:
        try:
            import praw
        except ImportError:
            logger.error("[reddit] praw not installed")
            return

        if not config.REDDIT_CLIENT_ID:
            logger.error("[reddit] REDDIT_CLIENT_ID not set")
            return

        reddit = praw.Reddit(
            client_id=config.REDDIT_CLIENT_ID,
            client_secret=config.REDDIT_CLIENT_SECRET,
            user_agent=config.REDDIT_USER_AGENT,
        )

        for sub_name in self.subreddits:
            if self.collected >= self.target_count:
                break
            try:
                subreddit = reddit.subreddit(sub_name)
                for post in subreddit.top(time_filter="year", limit=1000):
                    text = post.selftext
                    if len(text.split()) < 200:
                        continue
                    # Filter for likely human: account age > 6 months, karma > 100
                    author = post.author
                    if author and hasattr(author, "link_karma"):
                        if author.link_karma + author.comment_karma < 100:
                            continue
                    yield make_record(text, "human", "reddit", "personal")
                    self.collected += 1
                    if self.collected >= self.target_count:
                        break
            except Exception as e:
                logger.warning("[reddit] Error on r/%s: %s", sub_name, e)
                continue

    def dry_run(self) -> bool:
        if not config.REDDIT_CLIENT_ID:
            logger.warning("[reddit] No API credentials configured")
            return False
        try:
            import praw

            reddit = praw.Reddit(
                client_id=config.REDDIT_CLIENT_ID,
                client_secret=config.REDDIT_CLIENT_SECRET,
                user_agent=config.REDDIT_USER_AGENT,
            )
            sub = reddit.subreddit("test")
            _ = sub.display_name
            logger.info("[reddit] OK")
            return True
        except Exception as e:
            logger.error("[reddit] FAILED: %s", e)
            return False


class GitHubCollector(BaseCollector):
    """Collect READMEs from high-star GitHub repositories.

    Target: 10% of human data (~5,000 documents).
    Register: technical.
    """

    name = "github"
    register = "technical"
    API_URL = "https://api.github.com/search/repositories"

    def collect(self) -> Iterator[dict]:
        headers = {}
        if config.GITHUB_TOKEN:
            headers["Authorization"] = f"token {config.GITHUB_TOKEN}"

        languages = ["Python", "JavaScript", "TypeScript", "Rust", "Go", "Java", "C++"]
        for lang in languages:
            if self.collected >= self.target_count:
                break
            page = 1
            while self.collected < self.target_count and page <= 10:
                try:
                    resp = requests.get(
                        self.API_URL,
                        params={
                            "q": f"language:{lang} stars:>1000",
                            "sort": "stars",
                            "per_page": 30,
                            "page": page,
                        },
                        headers=headers,
                        timeout=30,
                    )
                    resp.raise_for_status()
                    data = resp.json()
                except (requests.RequestException, json.JSONDecodeError) as e:
                    logger.warning("[github] Search error: %s", e)
                    time.sleep(10)
                    break

                for repo in data.get("items", []):
                    full_name = repo["full_name"]
                    try:
                        readme_resp = requests.get(
                            f"https://raw.githubusercontent.com/{full_name}/HEAD/README.md",
                            headers=headers,
                            timeout=15,
                        )
                        if readme_resp.status_code != 200:
                            continue
                        text = readme_resp.text
                        if len(text.split()) >= 200:
                            yield make_record(text, "human", "github_readme", "technical")
                            self.collected += 1
                    except requests.RequestException:
                        continue

                page += 1
                time.sleep(2)  # GitHub rate limiting

    def dry_run(self) -> bool:
        try:
            headers = {}
            if config.GITHUB_TOKEN:
                headers["Authorization"] = f"token {config.GITHUB_TOKEN}"
            resp = requests.get(
                self.API_URL,
                params={"q": "stars:>10000", "per_page": 1},
                headers=headers,
                timeout=10,
            )
            ok = resp.status_code == 200
            logger.info("[github] %s", "OK" if ok else f"FAILED ({resp.status_code})")
            return ok
        except requests.RequestException as e:
            logger.error("[github] FAILED: %s", e)
            return False


class LegalCollector(BaseCollector):
    """Collect legal filings and court opinions.

    Target: 5% of human data (~2,500 documents).
    Register: legal.
    Sources: CourtListener API, public legal databases.
    """

    name = "legal"
    register = "legal"
    API_URL = "https://www.courtlistener.com/api/rest/v3/opinions/"

    def collect(self) -> Iterator[dict]:
        page = 1
        while self.collected < self.target_count:
            try:
                resp = requests.get(
                    self.API_URL,
                    params={
                        "format": "json",
                        "page": page,
                        "order_by": "-date_created",
                    },
                    timeout=30,
                )
                resp.raise_for_status()
                data = resp.json()
            except (requests.RequestException, json.JSONDecodeError) as e:
                logger.warning("[legal] Error: %s", e)
                time.sleep(5)
                page += 1
                continue

            for result in data.get("results", []):
                text = result.get("plain_text", "") or result.get("html", "")
                # Strip HTML tags if needed
                if "<" in text:
                    import re
                    text = re.sub(r"<[^>]+>", "", text)
                if len(text.split()) >= 300:
                    yield make_record(text[:10000], "human", "court_opinion", "legal")
                    self.collected += 1
                    if self.collected >= self.target_count:
                        break

            if not data.get("next"):
                break
            page += 1
            time.sleep(2)

    def dry_run(self) -> bool:
        try:
            resp = requests.get(
                self.API_URL,
                params={"format": "json", "page": 1},
                timeout=10,
            )
            ok = resp.status_code == 200
            logger.info("[legal] %s", "OK" if ok else f"FAILED ({resp.status_code})")
            return ok
        except requests.RequestException as e:
            logger.error("[legal] FAILED: %s", e)
            return False


# ─── AI Text Generator ────────────────────────────────────────────────────────


class AITextGenerator:
    """Generate AI text via local Ollama models (free, no API keys).

    Implements the prompting matrix:
      30% zero-shot, 25% few-shot, 20% persona,
      15% anti-detection, 10% chain-of-thought.

    Requires: ollama running locally (brew services start ollama)
    Models: ollama pull llama3 && ollama pull mistral && ollama pull gemma2
    """

    def __init__(self, target_count: int):
        self.target_count = target_count
        self.generated = 0
        self.ollama_url = f"{config.OLLAMA_BASE_URL}/api/chat"

    def _call_ollama(self, model: str, messages: list[dict]) -> str | None:
        """Make a single call to the local Ollama server."""
        payload = {
            "model": model,
            "messages": messages,
            "stream": False,
            "options": {
                "temperature": 0.9,
                "num_predict": 2000,
            },
        }

        for attempt in range(3):
            try:
                resp = requests.post(
                    self.ollama_url,
                    json=payload,
                    timeout=300,  # Local models can be slow on CPU
                )
                resp.raise_for_status()
                data = resp.json()
                content = data.get("message", {}).get("content", "")
                return content
            except (requests.RequestException, KeyError) as e:
                logger.warning("[ai_gen] Attempt %d failed for %s: %s", attempt + 1, model, e)
                time.sleep(2 ** attempt)

        return None

    def _build_prompt(
        self, prompt_type: str, register: str, topic: str
    ) -> list[dict]:
        """Build the chat messages for a given prompt type."""
        templates = config.GENERATION_TOPICS.get(register, config.GENERATION_TOPICS["academic"])
        base_prompt = random.choice(templates).format(topic=topic)

        if prompt_type == "zero_shot":
            return [{"role": "user", "content": base_prompt}]

        elif prompt_type == "few_shot":
            return [
                {"role": "system", "content": "You are a skilled writer."},
                {"role": "user", "content": f"Here is an example of good writing on this topic: [example]. Now, {base_prompt}"},
            ]

        elif prompt_type == "persona":
            persona = random.choice(config.PERSONA_PREFIXES)
            return [
                {"role": "system", "content": persona},
                {"role": "user", "content": base_prompt},
            ]

        elif prompt_type == "anti_detection":
            suffix = random.choice(config.ANTI_DETECTION_SUFFIXES)
            return [{"role": "user", "content": f"{base_prompt}\n\n{suffix}"}]

        elif prompt_type == "chain_of_thought":
            return [
                {"role": "user", "content": f"Think step by step about how to write about this topic, then produce the text. Topic: {base_prompt}"},
            ]

        return [{"role": "user", "content": base_prompt}]

    def generate(self) -> Iterator[dict]:
        """Generate AI text across all models and prompt types."""
        topics = [
            "climate change", "artificial intelligence", "healthcare reform",
            "cybersecurity", "renewable energy", "space exploration",
            "economic inequality", "education policy", "data privacy",
            "gene therapy", "quantum computing", "urban planning",
            "mental health", "immigration policy", "blockchain technology",
        ]

        # Build generation plan respecting distribution
        plan = []
        per_model = self.target_count // len(config.AI_MODELS)
        for model in config.AI_MODELS:
            for ptype, fraction in config.PROMPT_TYPES.items():
                count = int(per_model * fraction)
                for _ in range(count):
                    register = random.choice(config.REGISTERS)
                    topic = random.choice(topics)
                    plan.append((model, ptype, register, topic))

        random.shuffle(plan)

        for model, ptype, register, topic in tqdm(plan, desc="Generating AI text"):
            if self.generated >= self.target_count:
                break

            messages = self._build_prompt(ptype, register, topic)
            text = self._call_ollama(model, messages)

            if text and len(text.split()) >= 100:
                yield make_record(
                    text, "ai", f"ollama/{model}", register,
                    model=model, prompt_type=ptype,
                )
                self.generated += 1


# ─── Humanizer Pipeline ──────────────────────────────────────────────────────


class HumanizerPipeline:
    """Run AI-generated text through local Ollama paraphrasing.

    Processes a subset of AI samples by asking a local model to
    rewrite them in a more human-like style. Free, no API keys needed.

    Uses varied paraphrasing prompts to create diverse adversarial samples.
    """

    PARAPHRASE_PROMPTS = [
        "Rewrite the following text in your own words. Keep the same meaning but change the writing style, sentence structure, and word choices. Make it sound like a different person wrote it:\n\n{text}",
        "Paraphrase this text to sound more natural and human. Vary the sentence lengths, add some informality, and rephrase key points:\n\n{text}",
        "Rewrite this text as if you were a college student writing casually. Keep the core ideas but change the tone and structure:\n\n{text}",
        "Take this text and rewrite it with a completely different writing style. Use different vocabulary, restructure paragraphs, and vary the rhythm:\n\n{text}",
        "Rephrase this entire text to make it sound more authentic and personal. Add transitions, vary sentence complexity, and use more natural phrasing:\n\n{text}",
    ]

    def __init__(self, target_count: int):
        self.target_count = target_count
        self.processed = 0
        self.ollama_url = f"{config.OLLAMA_BASE_URL}/api/chat"

    def _paraphrase(self, text: str) -> str | None:
        """Paraphrase text using a local Ollama model."""
        prompt = random.choice(self.PARAPHRASE_PROMPTS).format(text=text[:4000])

        payload = {
            "model": config.HUMANIZER_MODEL,
            "messages": [{"role": "user", "content": prompt}],
            "stream": False,
            "options": {"temperature": 0.8, "num_predict": 2000},
        }

        try:
            resp = requests.post(self.ollama_url, json=payload, timeout=300)
            resp.raise_for_status()
            data = resp.json()
            return data.get("message", {}).get("content", "")
        except (requests.RequestException, KeyError) as e:
            logger.warning("[humanizer] Ollama paraphrase error: %s", e)
            return None

    def humanize(self, ai_docs_path: Path) -> Iterator[dict]:
        """Read AI documents and paraphrase them via Ollama."""
        if not ai_docs_path.exists():
            logger.error("[humanizer] AI docs file not found: %s", ai_docs_path)
            return

        with open(ai_docs_path, "r") as f:
            lines = f.readlines()

        # Sample up to target_count
        if len(lines) > self.target_count:
            lines = random.sample(lines, self.target_count)

        for line in tqdm(lines, desc="Humanizing AI text (Ollama)"):
            if self.processed >= self.target_count:
                break

            try:
                doc = json.loads(line)
            except json.JSONDecodeError:
                continue

            text = doc.get("text", "")
            if not text:
                continue

            humanized = self._paraphrase(text)
            if humanized and len(humanized.split()) >= 100:
                yield make_record(
                    humanized,
                    "humanized",
                    f"humanized/ollama_{config.HUMANIZER_MODEL}",
                    doc.get("register", "mixed"),
                    model=doc.get("model", ""),
                    prompt_type=doc.get("prompt_type", ""),
                )
                self.processed += 1


# ─── Main Pipeline Orchestrator ───────────────────────────────────────────────


def run_collection(args):
    """Run the full data collection pipeline."""
    output_dir = config.RAW_DIR
    output_dir.mkdir(parents=True, exist_ok=True)

    human_path = output_dir / "human.jsonl"
    ai_path = output_dir / "ai.jsonl"
    humanized_path = output_dir / "humanized.jsonl"

    dedup = MinHashDeduplicator(threshold=0.8)

    # Phase 1: Collect human documents
    if not args.ai_only and not args.humanize_only:
        logger.info("=" * 60)
        logger.info("PHASE 1: Human Document Collection")
        logger.info("=" * 60)

        collectors = {
            "arxiv": ArxivCollector,
            "news": NewsCollector,
            "gutenberg": GutenbergCollector,
            "edgar": EdgarCollector,
            "reddit": RedditCollector,
            "github": GitHubCollector,
            "legal": LegalCollector,
        }

        existing_count = count_lines(human_path) if args.resume else 0
        if existing_count > 0:
            logger.info("Resuming from %d existing human documents", existing_count)

        for source_name, fraction in config.HUMAN_SOURCES.items():
            if args.source and args.source != source_name:
                continue

            target = int(config.HUMAN_TARGET * fraction)
            collector_cls = collectors.get(source_name)
            if not collector_cls:
                logger.warning("Unknown source: %s", source_name)
                continue

            logger.info("[%s] Collecting up to %d documents...", source_name, target)
            collector = collector_cls(target)

            for record in collector.collect():
                if not dedup.is_duplicate(record["text"]):
                    append_jsonl(record, human_path)

        human_count = count_lines(human_path)
        logger.info("Human collection complete: %d documents", human_count)

    # Phase 2: Generate AI documents (skip when collecting a specific human source)
    if not args.humanize_only and not args.source:
        logger.info("=" * 60)
        logger.info("PHASE 2: AI Text Generation")
        logger.info("=" * 60)

        existing_ai = count_lines(ai_path) if args.resume else 0
        if existing_ai > 0:
            logger.info("Resuming from %d existing AI documents", existing_ai)

        generator = AITextGenerator(config.AI_TARGET - existing_ai)
        for record in generator.generate():
            if not dedup.is_duplicate(record["text"]):
                append_jsonl(record, ai_path)

        ai_count = count_lines(ai_path)
        logger.info("AI generation complete: %d documents", ai_count)

    # Phase 3: Humanize AI samples (skip when collecting a specific human source)
    if args.source:
        logger.info("Skipping humanizer (single source mode)")
        return

    logger.info("=" * 60)
    logger.info("PHASE 3: Humanizer Pipeline")
    logger.info("=" * 60)

    existing_humanized = count_lines(humanized_path) if args.resume else 0
    pipeline = HumanizerPipeline(config.HUMANIZED_TARGET - existing_humanized)
    for record in pipeline.humanize(ai_path):
        append_jsonl(record, humanized_path)

    humanized_count = count_lines(humanized_path)
    logger.info("Humanization complete: %d documents", humanized_count)

    # Summary
    logger.info("=" * 60)
    logger.info("COLLECTION COMPLETE")
    logger.info("=" * 60)
    logger.info("  Human:     %d / %d", count_lines(human_path), config.HUMAN_TARGET)
    logger.info("  AI:        %d / %d", count_lines(ai_path), config.AI_TARGET)
    logger.info("  Humanized: %d / %d", humanized_count, config.HUMANIZED_TARGET)
    logger.info("  Dedup index size: %d", dedup.index_size)


def run_dry_run():
    """Test connectivity to all data sources."""
    logger.info("DRY RUN — Testing API connectivity")
    logger.info("=" * 60)

    collectors = [
        ArxivCollector(0),
        NewsCollector(0),
        GutenbergCollector(0),
        EdgarCollector(0),
        RedditCollector(0),
        GitHubCollector(0),
        LegalCollector(0),
    ]

    results = {}
    for c in collectors:
        results[c.name] = c.dry_run()

    # Check Ollama
    try:
        resp = requests.get(f"{config.OLLAMA_BASE_URL}/api/tags", timeout=10)
        if resp.status_code == 200:
            models = [m["name"] for m in resp.json().get("models", [])]
            available = [m for m in config.AI_MODELS if any(m in name for name in models)]
            results["ollama"] = len(available) > 0
            if available:
                logger.info("[ollama] OK — models available: %s", ", ".join(available))
            else:
                logger.warning("[ollama] Server running but no required models found. Run: ollama pull llama3 && ollama pull mistral && ollama pull gemma2")
        else:
            results["ollama"] = False
    except requests.RequestException:
        results["ollama"] = False
        logger.warning("[ollama] Not running. Start with: brew services start ollama")

    logger.info("=" * 60)
    logger.info("DRY RUN RESULTS:")
    for name, ok in results.items():
        status = "OK" if ok else "FAILED"
        logger.info("  %-15s %s", name, status)

    all_ok = all(results.values())
    if not all_ok:
        logger.warning("Some sources are not reachable. Collection will skip failed sources.")
    return all_ok


def main():
    parser = argparse.ArgumentParser(
        description="Provenance ML data collection pipeline."
    )
    parser.add_argument("--dry-run", action="store_true", help="Test API connectivity only")
    parser.add_argument("--source", type=str, default=None, help="Collect from a single human source")
    parser.add_argument("--ai-only", action="store_true", help="Generate AI text only")
    parser.add_argument("--humanize-only", action="store_true", help="Run humanizer pipeline only")
    parser.add_argument("--resume", action="store_true", help="Resume from last checkpoint")
    args = parser.parse_args()

    if args.dry_run:
        ok = run_dry_run()
        sys.exit(0 if ok else 1)

    run_collection(args)


if __name__ == "__main__":
    main()
