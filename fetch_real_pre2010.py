import urllib.request
import xml.etree.ElementTree as ET
import json
import os
import time
import re

BASE = '/Users/Admin/Projects/provenance/test_suite_pre2010'
os.makedirs(BASE, exist_ok=True)

manifest = {}
total = 0

# ============ 1. arXiv PAPERS 2000-2010 (40 papers across fields) ============
print('=== Fetching arXiv papers 2000-2010 ===')
os.makedirs(f'{BASE}/academic', exist_ok=True)
arxiv_queries = [
    ('cs.AI', 'http://export.arxiv.org/api/query?search_query=cat:cs.AI+AND+submittedDate:[200001010000+TO+200512312359]&start=0&max_results=8&sortBy=relevance'),
    ('physics', 'http://export.arxiv.org/api/query?search_query=cat:physics.gen-ph+AND+submittedDate:[200001010000+TO+200612312359]&start=0&max_results=8&sortBy=relevance'),
    ('math', 'http://export.arxiv.org/api/query?search_query=cat:math.CO+AND+submittedDate:[200101010000+TO+200712312359]&start=0&max_results=8&sortBy=relevance'),
    ('biology', 'http://export.arxiv.org/api/query?search_query=cat:q-bio+AND+submittedDate:[200401010000+TO+200912312359]&start=0&max_results=8&sortBy=relevance'),
    ('astro', 'http://export.arxiv.org/api/query?search_query=cat:astro-ph+AND+submittedDate:[200001010000+TO+200512312359]&start=0&max_results=8&sortBy=relevance'),
]
ns = {'atom': 'http://www.w3.org/2005/Atom'}
acad_count = 0
for field, url in arxiv_queries:
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        resp = urllib.request.urlopen(req, timeout=20)
        root = ET.fromstring(resp.read())
        for entry in root.findall('atom:entry', ns):
            title = entry.find('atom:title', ns).text.strip().replace('\n', ' ')
            abstract = entry.find('atom:summary', ns).text.strip().replace('\n', ' ')
            published = entry.find('atom:published', ns).text[:4]
            if len(abstract) > 200:
                acad_count += 1
                total += 1
                fname = f'{BASE}/academic/arxiv_{field}_{acad_count:03d}_{published}.txt'
                with open(fname, 'w') as f:
                    f.write(f'{title}\n\n{abstract}')
                print(f'  [{total}] arxiv_{field}_{acad_count:03d} ({published}): {title[:60]}...')
        time.sleep(3)  # be nice to arxiv
    except Exception as e:
        print(f'  Error fetching {field}: {e}')
print(f'  Academic total: {acad_count}')

# ============ 2. Project Gutenberg - REAL literary texts pre-1930 ============
print('\n=== Fetching Project Gutenberg texts ===')
os.makedirs(f'{BASE}/literary', exist_ok=True)

gutenberg_books = [
    ('pg1342', 'https://www.gutenberg.org/cache/epub/1342/pg1342.txt', 'Pride and Prejudice - Austen 1813'),
    ('pg11', 'https://www.gutenberg.org/cache/epub/11/pg11.txt', 'Alice in Wonderland - Carroll 1865'),
    ('pg1661', 'https://www.gutenberg.org/cache/epub/1661/pg1661.txt', 'Sherlock Holmes - Doyle 1892'),
    ('pg84', 'https://www.gutenberg.org/cache/epub/84/pg84.txt', 'Frankenstein - Shelley 1818'),
    ('pg98', 'https://www.gutenberg.org/cache/epub/98/pg98.txt', 'Tale of Two Cities - Dickens 1859'),
    ('pg1232', 'https://www.gutenberg.org/cache/epub/1232/pg1232.txt', 'The Prince - Machiavelli 1532'),
    ('pg76', 'https://www.gutenberg.org/cache/epub/76/pg76.txt', 'Huckleberry Finn - Twain 1884'),
    ('pg2701', 'https://www.gutenberg.org/cache/epub/2701/pg2701.txt', 'Moby Dick - Melville 1851'),
    ('pg1400', 'https://www.gutenberg.org/cache/epub/1400/pg1400.txt', 'Great Expectations - Dickens 1861'),
    ('pg174', 'https://www.gutenberg.org/cache/epub/174/pg174.txt', 'Picture of Dorian Gray - Wilde 1890'),
    ('pg16', 'https://www.gutenberg.org/cache/epub/16/pg16.txt', 'Peter Pan - Barrie 1911'),
    ('pg1080', 'https://www.gutenberg.org/cache/epub/1080/pg1080.txt', 'A Modest Proposal - Swift 1729'),
    ('pg2591', 'https://www.gutenberg.org/cache/epub/2591/pg2591.txt', 'Grimms Fairy Tales'),
    ('pg345', 'https://www.gutenberg.org/cache/epub/345/pg345.txt', 'Dracula - Stoker 1897'),
    ('pg1952', 'https://www.gutenberg.org/cache/epub/1952/pg1952.txt', 'The Yellow Wallpaper - Gilman 1892'),
]

lit_count = 0
for book_id, url, title in gutenberg_books:
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        resp = urllib.request.urlopen(req, timeout=30)
        text = resp.read().decode('utf-8', errors='ignore')
        
        # Extract ~1000 words from the middle of the book (skip headers/footers)
        lines = text.split('\n')
        # Find the start of actual content (after Gutenberg header)
        start_idx = 0
        for i, line in enumerate(lines):
            if '*** START' in line or '***START' in line:
                start_idx = i + 1
                break
        
        # Find end of content
        end_idx = len(lines)
        for i, line in enumerate(lines):
            if '*** END' in line or '***END' in line:
                end_idx = i
                break
        
        content_lines = lines[start_idx:end_idx]
        content = '\n'.join(content_lines).strip()
        
        # Take ~1500 words from 20% into the text (past any preamble)
        words = content.split()
        if len(words) > 2000:
            start_word = len(words) // 5
            excerpt = ' '.join(words[start_word:start_word+1500])
        else:
            excerpt = ' '.join(words[:1500])
        
        if len(excerpt) > 500:
            lit_count += 1
            total += 1
            fname = f'{BASE}/literary/{book_id}_{lit_count:03d}.txt'
            with open(fname, 'w') as f:
                f.write(f'{title}\n\n{excerpt}')
            print(f'  [{total}] {book_id}: {title}')
        
        time.sleep(2)
    except Exception as e:
        print(f'  Error fetching {title}: {e}')
print(f'  Literary total: {lit_count}')

# ============ 3. US Supreme Court opinions (pre-2010) ============
print('\n=== Fetching legal texts ===')
os.makedirs(f'{BASE}/legal', exist_ok=True)

# These are real legal texts - using Cornell LII / public domain sources
legal_urls = [
    ('us_const_amend_1_10', 'https://www.gutenberg.org/cache/epub/5/pg5.txt', 'US Constitution - Bill of Rights 1791'),
]

# We'll also use Gutenberg legal/political texts
legal_gutenberg = [
    ('pg1497', 'https://www.gutenberg.org/cache/epub/1497/pg1497.txt', 'Republic - Plato'),
    ('pg3207', 'https://www.gutenberg.org/cache/epub/3207/pg3207.txt', 'Leviathan - Hobbes 1651'),
    ('pg7370', 'https://www.gutenberg.org/cache/epub/7370/pg7370.txt', 'Second Treatise of Government - Locke 1689'),
    ('pg5', 'https://www.gutenberg.org/cache/epub/5/pg5.txt', 'US Constitution'),
    ('pg815', 'https://www.gutenberg.org/cache/epub/815/pg815.txt', 'Democracy in America - Tocqueville 1835'),
    ('pg1404', 'https://www.gutenberg.org/cache/epub/1404/pg1404.txt', 'Common Sense - Paine 1776'),
]

legal_count = 0
for book_id, url, title in legal_gutenberg:
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        resp = urllib.request.urlopen(req, timeout=30)
        text = resp.read().decode('utf-8', errors='ignore')
        
        lines = text.split('\n')
        start_idx = 0
        for i, line in enumerate(lines):
            if '*** START' in line or '***START' in line:
                start_idx = i + 1
                break
        end_idx = len(lines)
        for i, line in enumerate(lines):
            if '*** END' in line or '***END' in line:
                end_idx = i
                break
        
        content = '\n'.join(lines[start_idx:end_idx]).strip()
        words = content.split()
        if len(words) > 2000:
            start_word = len(words) // 5
            excerpt = ' '.join(words[start_word:start_word+1500])
        else:
            excerpt = ' '.join(words[:1500])
        
        if len(excerpt) > 500:
            legal_count += 1
            total += 1
            fname = f'{BASE}/legal/{book_id}_{legal_count:03d}.txt'
            with open(fname, 'w') as f:
                f.write(f'{title}\n\n{excerpt}')
            print(f'  [{total}] {book_id}: {title}')
        
        time.sleep(2)
    except Exception as e:
        print(f'  Error fetching {title}: {e}')
print(f'  Legal/Political total: {legal_count}')

# ============ 4. Scientific texts from Gutenberg (pre-1930) ============
print('\n=== Fetching scientific/technical texts ===')
os.makedirs(f'{BASE}/scientific', exist_ok=True)

science_gutenberg = [
    ('pg4217', 'https://www.gutenberg.org/cache/epub/4217/pg4217.txt', 'A Brief History of Time concept - Einstein Relativity 1916'),
    ('pg28054', 'https://www.gutenberg.org/cache/epub/28054/pg28054.txt', 'The Origin of Species - Darwin 1859'),
    ('pg36276', 'https://www.gutenberg.org/cache/epub/36276/pg36276.txt', 'Principia Mathematica Intro - Newton'),
    ('pg14725', 'https://www.gutenberg.org/cache/epub/14725/pg14725.txt', 'Interpretation of Dreams - Freud 1899'),
    ('pg5001', 'https://www.gutenberg.org/cache/epub/5001/pg5001.txt', 'Art of War - Sun Tzu'),
    ('pg4363', 'https://www.gutenberg.org/cache/epub/4363/pg4363.txt', 'Beyond Good and Evil - Nietzsche'),
    ('pg1228', 'https://www.gutenberg.org/cache/epub/1228/pg1228.txt', 'On Liberty - Mill 1859'),
    ('pg3300', 'https://www.gutenberg.org/cache/epub/3300/pg3300.txt', 'An Inquiry into Wealth of Nations - Smith 1776'),
]

sci_count = 0
for book_id, url, title in science_gutenberg:
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        resp = urllib.request.urlopen(req, timeout=30)
        text = resp.read().decode('utf-8', errors='ignore')
        
        lines = text.split('\n')
        start_idx = 0
        for i, line in enumerate(lines):
            if '*** START' in line or '***START' in line:
                start_idx = i + 1
                break
        end_idx = len(lines)
        for i, line in enumerate(lines):
            if '*** END' in line or '***END' in line:
                end_idx = i
                break
        
        content = '\n'.join(lines[start_idx:end_idx]).strip()
        words = content.split()
        if len(words) > 2000:
            start_word = len(words) // 5
            excerpt = ' '.join(words[start_word:start_word+1500])
        else:
            excerpt = ' '.join(words[:1500])
        
        if len(excerpt) > 500:
            sci_count += 1
            total += 1
            fname = f'{BASE}/scientific/{book_id}_{sci_count:03d}.txt'
            with open(fname, 'w') as f:
                f.write(f'{title}\n\n{excerpt}')
            print(f'  [{total}] {book_id}: {title}')
        
        time.sleep(2)
    except Exception as e:
        print(f'  Error fetching {title}: {e}')
print(f'  Scientific total: {sci_count}')

# ============ 5. More arXiv from different eras ============
print('\n=== Fetching more arXiv 2006-2010 ===')
os.makedirs(f'{BASE}/academic_late', exist_ok=True)

late_queries = [
    ('cs.CL', 'http://export.arxiv.org/api/query?search_query=cat:cs.CL+AND+submittedDate:[200601010000+TO+201012312359]&start=0&max_results=10&sortBy=relevance'),
    ('cs.LG', 'http://export.arxiv.org/api/query?search_query=cat:cs.LG+AND+submittedDate:[200601010000+TO+201012312359]&start=0&max_results=10&sortBy=relevance'),
]

late_count = 0
for field, url in late_queries:
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        resp = urllib.request.urlopen(req, timeout=20)
        root = ET.fromstring(resp.read())
        for entry in root.findall('atom:entry', ns):
            title = entry.find('atom:title', ns).text.strip().replace('\n', ' ')
            abstract = entry.find('atom:summary', ns).text.strip().replace('\n', ' ')
            published = entry.find('atom:published', ns).text[:4]
            if len(abstract) > 200 and int(published) <= 2010:
                late_count += 1
                total += 1
                fname = f'{BASE}/academic_late/arxiv_{field}_{late_count:03d}_{published}.txt'
                with open(fname, 'w') as f:
                    f.write(f'{title}\n\n{abstract}')
                print(f'  [{total}] arxiv_{field}_{late_count:03d} ({published}): {title[:60]}...')
        time.sleep(3)
    except Exception as e:
        print(f'  Error fetching {field}: {e}')
print(f'  Academic late total: {late_count}')

# ============ MANIFEST ============
print(f'\n=== MANIFEST ===')
manifest = {'total': total, 'categories': {}}
for cat in os.listdir(BASE):
    cat_path = os.path.join(BASE, cat)
    if os.path.isdir(cat_path):
        files = sorted([f for f in os.listdir(cat_path) if f.endswith('.txt')])
        if files:
            manifest['categories'][cat] = {'count': len(files), 'files': files}
            print(f'  {cat}: {len(files)} files')

with open(f'{BASE}/manifest.json', 'w') as f:
    json.dump(manifest, f, indent=2)

print(f'\nTOTAL: {total} real pre-2010 documents')
print('=== DONE ===')
