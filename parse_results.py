import json
import os
import csv

RESULTS = '/Users/Admin/Projects/provenance/test_results_full'
OUTPUT = os.path.join(RESULTS, 'analysis')
os.makedirs(OUTPUT, exist_ok=True)

categories = ['academic', 'literary', 'legal', 'journalism', 'technical', 'personal', 'business', 'medical', 'political', 'mixed']

all_results = []

for cat in categories:
    cat_path = os.path.join(RESULTS, cat)
    if not os.path.isdir(cat_path):
        continue

    for fname in sorted(os.listdir(cat_path)):
        if not fname.endswith('_analyze.json'):
            continue

        fpath = os.path.join(cat_path, fname)
        try:
            content = open(fpath).read()
            start = content.find('{')
            if start < 0:
                print(f'  SKIP {fname}: no JSON')
                continue
            data = json.loads(content[start:])
        except Exception as e:
            print(f'  SKIP {fname}: {e}')
            continue

        basename = fname.replace('_analyze.json', '')

        # Register
        reg = data.get('register', {})
        reg_label = reg.get('label', 'Unknown')
        reg_conf = reg.get('confidence', 0)
        feat = reg.get('feature_scores', {})

        # ACS
        acs = data.get('acs', {})
        acs_score = acs.get('score', 0)
        acs_margin = acs.get('margin', 0)

        # PII
        pii = data.get('pii', {})
        pii_score = pii.get('score', 0)
        pii_level = pii.get('level', '')

        # Baseline
        bl = data.get('baseline_report', {})
        bl_expected = bl.get('expected_count', 0)
        bl_atypical = bl.get('atypical_count', 0)
        bl_anomalous = bl.get('anomalous_count', 0)

        # Anomalies
        anom = data.get('anomaly_report', {})
        anom_high = anom.get('high_count', 0)
        anom_medium = anom.get('medium_count', 0)
        anom_low = anom.get('low_count', 0)

        # Metrics from analysis_result
        ar = data.get('analysis_result', {})
        lex = ar.get('lexical', {})
        syn = ar.get('syntactic', {})

        result = {
            'file': basename,
            'category': cat,
            'register_label': reg_label,
            'register_confidence': round(reg_conf, 3),
            'acs_score': round(acs_score, 1),
            'acs_margin': round(acs_margin, 1),
            'pii_score': round(pii_score, 1),
            'pii_level': pii_level,
            'signals_expected': bl_expected,
            'signals_atypical': bl_atypical,
            'signals_anomalous': bl_anomalous,
            'anomaly_high': anom_high,
            'anomaly_medium': anom_medium,
            'anomaly_low': anom_low,
            'total_words': lex.get('total_words', 0),
            'ttr': round(lex.get('type_token_ratio', 0), 3),
            'avg_word_length': round(lex.get('avg_word_length', 0), 2),
            'avg_sentence_length': round(feat.get('avg_sentence_length', 0), 2),
            'passive_voice_ratio': round(feat.get('passive_voice_ratio', 0), 3),
            'flesch_kincaid': round(feat.get('reading_grade', 0), 2),
            'formality': round(feat.get('formality_score', 0), 3),
        }
        all_results.append(result)

    print(f'{cat}: {sum(1 for r in all_results if r["category"]==cat)} parsed')

# Save JSON + CSV
with open(os.path.join(OUTPUT, 'all_results.json'), 'w') as f:
    json.dump(all_results, f, indent=2)

if all_results:
    with open(os.path.join(OUTPUT, 'all_results.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=all_results[0].keys())
        w.writeheader()
        w.writerows(all_results)

# ============ ANALYSIS ============
print(f'\nTotal: {len(all_results)} documents')

# Register detection
print('\n=== REGISTER DETECTION BY CATEGORY ===')
reg_map = {}
for r in all_results:
    cat = r['category']
    reg = r['register_label']
    reg_map.setdefault(cat, {})
    reg_map[cat][reg] = reg_map[cat].get(reg, 0) + 1

for cat in categories:
    if cat in reg_map:
        print(f'\n  {cat.upper()}:')
        for reg, cnt in sorted(reg_map[cat].items(), key=lambda x: -x[1]):
            print(f'    {reg}: {cnt}')

# ACS
print('\n=== ACS SCORES ===')
for cat in categories:
    scores = [r['acs_score'] for r in all_results if r['category'] == cat]
    if scores:
        print(f'  {cat}: avg={sum(scores)/len(scores):.1f}, min={min(scores):.0f}, max={max(scores):.0f}')

# Signals
print('\n=== SIGNAL CLASSIFICATION ===')
for cat in categories:
    exp = sum(r['signals_expected'] for r in all_results if r['category'] == cat)
    aty = sum(r['signals_atypical'] for r in all_results if r['category'] == cat)
    ano = sum(r['signals_anomalous'] for r in all_results if r['category'] == cat)
    total = exp + aty + ano
    if total:
        print(f'  {cat}: expected={exp} atypical={aty} anomalous={ano} ({ano/(total)*100:.0f}% anomalous)')

# Register accuracy
print('\n=== REGISTER ACCURACY (most common per category) ===')
expected_registers = {
    'academic': 'Academic',
    'literary': 'Literary',
    'legal': 'Legal',
    'journalism': 'Journal',
    'technical': 'Technical',
    'personal': 'Personal',
    'business': 'Business',
    'medical': 'Medical',
    'political': 'Political',
    'mixed': 'AI',
}
for cat in categories:
    if cat in reg_map:
        top_reg, top_cnt = max(reg_map[cat].items(), key=lambda x: x[1])
        total_in_cat = sum(reg_map[cat].values())
        pct = top_cnt / total_in_cat * 100
        expected = expected_registers.get(cat, '?')
        match = 'YES' if expected.lower() in top_reg.lower() else 'NO'
        print(f'  {cat} -> {top_reg} ({pct:.0f}%) [expected contains "{expected}": {match}]')

# Formality spectrum
print('\n=== FORMALITY SPECTRUM ===')
for cat in categories:
    vals = [r['formality'] for r in all_results if r['category'] == cat]
    if vals:
        print(f'  {cat}: avg={sum(vals)/len(vals):.3f} (range {min(vals):.3f}-{max(vals):.3f})')

# Save full report
lines = ['PROVENANCE 109-DOCUMENT TEST SUITE REPORT', '=' * 60, '']
lines.append(f'Total documents: {len(all_results)}')
lines.append(f'Categories: {len(categories)}')
lines.append('')

for cat in categories:
    lines.append(f'\n{"="*50}')
    lines.append(f'CATEGORY: {cat.upper()}')
    lines.append(f'{"="*50}')
    for r in [x for x in all_results if x['category'] == cat]:
        lines.append(f'\n  {r["file"]}')
        lines.append(f'    Register: {r["register_label"]} ({r["register_confidence"]})')
        lines.append(f'    ACS: {r["acs_score"]} (+/-{r["acs_margin"]})  PII: {r["pii_score"]} ({r["pii_level"]})')
        lines.append(f'    Words: {r["total_words"]}  TTR: {r["ttr"]}  Formality: {r["formality"]}')
        lines.append(f'    FK Grade: {r["flesch_kincaid"]}  Passive: {r["passive_voice_ratio"]}  AvgSentLen: {r["avg_sentence_length"]}')
        lines.append(f'    Signals: {r["signals_expected"]}exp/{r["signals_atypical"]}aty/{r["signals_anomalous"]}ano')
        lines.append(f'    Anomalies: {r["anomaly_high"]}H/{r["anomaly_medium"]}M/{r["anomaly_low"]}L')

with open(os.path.join(OUTPUT, 'full_report.txt'), 'w') as f:
    f.write('\n'.join(lines))

print(f'\nSaved to {OUTPUT}/')
print('=== DONE ===')
