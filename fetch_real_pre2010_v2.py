import urllib.request
import xml.etree.ElementTree as ET
import json
import os
import time
import ssl

# Disable SSL verification (macOS Python cert issue)
ssl_ctx = ssl.create_default_context()
ssl_ctx.check_hostname = False
ssl_ctx.verify_mode = ssl.CERT_NONE

BASE = '/Users/Admin/Projects/provenance/test_suite_pre2010'
os.makedirs(BASE, exist_ok=True)
total = 0

def fetch(url):
    req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
    return urllib.request.urlopen(req, timeout=30, context=ssl_ctx)

def extract_gutenberg(text, max_words=1500):
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
        return ' '.join(words[start_word:start_word+max_words])
    return ' '.join(words[:max_words])

# ============ 1. arXiv 2000-2010 (40 papers) ============
print('=== arXiv papers 2000-2010 ===')
os.makedirs(f'{BASE}/academic', exist_ok=True)
ns = {'atom': 'http://www.w3.org/2005/Atom'}
arxiv_queries = [
    ('cs.AI', 'http://export.arxiv.org/api/query?search_query=cat:cs.AI+AND+submittedDate:[200001010000+TO+200512312359]&start=0&max_results=8'),
    ('physics', 'http://export.arxiv.org/api/query?search_query=cat:physics.gen-ph+AND+submittedDate:[200001010000+TO+200612312359]&start=0&max_results=8'),
    ('math', 'http://export.arxiv.org/api/query?search_query=cat:math.CO+AND+submittedDate:[200101010000+TO+200712312359]&start=0&max_results=8'),
    ('bio', 'http://export.arxiv.org/api/query?search_query=cat:q-bio+AND+submittedDate:[200401010000+TO+200912312359]&start=0&max_results=8'),
    ('astro', 'http://export.arxiv.org/api/query?search_query=cat:astro-ph+AND+submittedDate:[200001010000+TO+200512312359]&start=0&max_results=8'),
]
acad_count = 0
for field, url in arxiv_queries:
    try:
        resp = fetch(url)
        root = ET.fromstring(resp.read())
        for entry in root.findall('atom:entry', ns):
            title = entry.find('atom:title', ns).text.strip().replace('\n', ' ')
            abstract = entry.find('atom:summary', ns).text.strip().replace('\n', ' ')
            published = entry.find('atom:published', ns).text[:4]
            if len(abstract) > 200:
                acad_count += 1
                total += 1
                with open(f'{BASE}/academic/arxiv_{acad_count:03d}_{field}_{published}.txt', 'w') as f:
                    f.write(f'{title}\n\n{abstract}')
                print(f'  [{total}] {field} ({published}): {title[:55]}...')
        time.sleep(3)
    except Exception as e:
        print(f'  Error {field}: {e}')
print(f'  Total academic: {acad_count}')

# ============ 2. Gutenberg Literary (15 books) ============
print('\n=== Project Gutenberg literary texts ===')
os.makedirs(f'{BASE}/literary', exist_ok=True)
gutenberg = [
    (1342, 'Pride and Prejudice - Austen 1813'),
    (11, 'Alice in Wonderland - Carroll 1865'),
    (1661, 'Sherlock Holmes - Doyle 1892'),
    (84, 'Frankenstein - Shelley 1818'),
    (98, 'Tale of Two Cities - Dickens 1859'),
    (76, 'Huckleberry Finn - Twain 1884'),
    (2701, 'Moby Dick - Melville 1851'),
    (1400, 'Great Expectations - Dickens 1861'),
    (174, 'Dorian Gray - Wilde 1890'),
    (16, 'Peter Pan - Barrie 1911'),
    (345, 'Dracula - Stoker 1897'),
    (1952, 'Yellow Wallpaper - Gilman 1892'),
    (2591, 'Grimms Fairy Tales'),
    (1080, 'Modest Proposal - Swift 1729'),
    (120, 'Treasure Island - Stevenson 1883'),
]
lit_count = 0
for pg_id, title in gutenberg:
    try:
        url = f'https://www.gutenberg.org/cache/epub/{pg_id}/pg{pg_id}.txt'
        text = fetch(url).read().decode('utf-8', errors='ignore')
        excerpt = extract_gutenberg(text)
        if len(excerpt) > 500:
            lit_count += 1
            total += 1
            with open(f'{BASE}/literary/gutenberg_{lit_count:03d}_pg{pg_id}.txt', 'w') as f:
                f.write(f'{title}\n\n{excerpt}')
            print(f'  [{total}] pg{pg_id}: {title}')
        time.sleep(2)
    except Exception as e:
        print(f'  Error {title}: {e}')
print(f'  Total literary: {lit_count}')

# ============ 3. Gutenberg Philosophy/Legal/Political (8) ============
print('\n=== Gutenberg philosophy/political texts ===')
os.makedirs(f'{BASE}/political', exist_ok=True)
political = [
    (1497, 'Republic - Plato'),
    (3207, 'Leviathan - Hobbes 1651'),
    (7370, 'Second Treatise - Locke 1689'),
    (5, 'US Constitution 1787'),
    (1404, 'Common Sense - Paine 1776'),
    (1228, 'On Liberty - Mill 1859'),
    (815, 'Democracy in America - Tocqueville'),
    (4363, 'Beyond Good and Evil - Nietzsche'),
]
pol_count = 0
for pg_id, title in political:
    try:
        url = f'https://www.gutenberg.org/cache/epub/{pg_id}/pg{pg_id}.txt'
        text = fetch(url).read().decode('utf-8', errors='ignore')
        excerpt = extract_gutenberg(text)
        if len(excerpt) > 500:
            pol_count += 1
            total += 1
            with open(f'{BASE}/political/gutenberg_{pol_count:03d}_pg{pg_id}.txt', 'w') as f:
                f.write(f'{title}\n\n{excerpt}')
            print(f'  [{total}] pg{pg_id}: {title}')
        time.sleep(2)
    except Exception as e:
        print(f'  Error {title}: {e}')
print(f'  Total political: {pol_count}')

# ============ 4. Gutenberg Scientific (8) ============
print('\n=== Gutenberg scientific texts ===')
os.makedirs(f'{BASE}/scientific', exist_ok=True)
science = [
    (28054, 'Origin of Species - Darwin 1859'),
    (14725, 'Interpretation of Dreams - Freud 1899'),
    (5001, 'Art of War - Sun Tzu'),
    (3300, 'Wealth of Nations - Smith 1776'),
    (4217, 'Relativity - Einstein 1916'),
    (22381, 'Euclid Elements'),
    (33504, 'Descent of Man - Darwin'),
    (10616, 'Principia Ethica - Moore 1903'),
]
sci_count = 0
for pg_id, title in science:
    try:
        url = f'https://www.gutenberg.org/cache/epub/{pg_id}/pg{pg_id}.txt'
        text = fetch(url).read().decode('utf-8', errors='ignore')
        excerpt = extract_gutenberg(text)
        if len(excerpt) > 500:
            sci_count += 1
            total += 1
            with open(f'{BASE}/scientific/gutenberg_{sci_count:03d}_pg{pg_id}.txt', 'w') as f:
                f.write(f'{title}\n\n{excerpt}')
            print(f'  [{total}] pg{pg_id}: {title}')
        time.sleep(2)
    except Exception as e:
        print(f'  Error {title}: {e}')
print(f'  Total scientific: {sci_count}')

# ============ 5. More arXiv 2006-2010 (20) ============
print('\n=== arXiv 2006-2010 ===')
os.makedirs(f'{BASE}/academic_late', exist_ok=True)
late_queries = [
    ('cs.CL', 'http://export.arxiv.org/api/query?search_query=cat:cs.CL+AND+submittedDate:[200601010000+TO+201012312359]&start=0&max_results=10'),
    ('cs.LG', 'http://export.arxiv.org/api/query?search_query=cat:cs.LG+AND+submittedDate:[200601010000+TO+201012312359]&start=0&max_results=10'),
]
late_count = 0
for field, url in late_queries:
    try:
        resp = fetch(url)
        root = ET.fromstring(resp.read())
        for entry in root.findall('atom:entry', ns):
            title = entry.find('atom:title', ns).text.strip().replace('\n', ' ')
            abstract = entry.find('atom:summary', ns).text.strip().replace('\n', ' ')
            published = entry.find('atom:published', ns).text[:4]
            if len(abstract) > 200 and int(published) <= 2010:
                late_count += 1
                total += 1
                with open(f'{BASE}/academic_late/arxiv_{late_count:03d}_{field}_{published}.txt', 'w') as f:
                    f.write(f'{title}\n\n{abstract}')
                print(f'  [{total}] {field} ({published}): {title[:55]}...')
        time.sleep(3)
    except Exception as e:
        print(f'  Error {field}: {e}')
print(f'  Total academic late: {late_count}')

# ============ MANIFEST ============
print(f'\n=== SUMMARY ===')
manifest = {'total': total, 'categories': {}}
for cat in sorted(os.listdir(BASE)):
    cat_path = os.path.join(BASE, cat)
    if os.path.isdir(cat_path):
        files = sorted([f for f in os.listdir(cat_path) if f.endswith('.txt')])
        if files:
            manifest['categories'][cat] = {'count': len(files), 'files': files}
            print(f'  {cat}: {len(files)}')
with open(f'{BASE}/manifest.json', 'w') as f:
    json.dump(manifest, f, indent=2)
print(f'\nTOTAL: {total} real pre-2010 documents')
