#!/bin/bash
CLI=~/Projects/provenance/target/release/provenance
SUITE=~/Projects/provenance/test_suite
RESULTS=~/Projects/provenance/test_results_full
mkdir -p $RESULTS

echo "=== Provenance Batch Test — $(date) ==="
echo "CLI: $CLI"
echo ""

TOTAL=0
SUCCESS=0
FAIL=0

for category in academic literary legal journalism technical personal business medical political mixed; do
    echo ""
    echo "========== CATEGORY: $category =========="
    mkdir -p $RESULTS/$category
    
    for file in $SUITE/$category/*.txt; do
        if [ ! -f "$file" ]; then continue; fi
        basename=$(basename "$file" .txt)
        TOTAL=$((TOTAL + 1))
        
        echo -n "  [$TOTAL] $basename... "
        
        # Run analyze with JSON output
        output=$($CLI analyze --file "$file" --format json 2>&1)
        exit_code=$?
        
        if [ $exit_code -eq 0 ]; then
            echo "$output" > "$RESULTS/$category/${basename}_analyze.json"
            SUCCESS=$((SUCCESS + 1))
            echo "OK"
        else
            echo "$output" > "$RESULTS/$category/${basename}_error.txt"
            FAIL=$((FAIL + 1))
            echo "FAIL (exit $exit_code)"
        fi
    done
done

echo ""
echo "========== SUMMARY =========="
echo "Total: $TOTAL"
echo "Success: $SUCCESS"
echo "Failed: $FAIL"
echo "Results saved to: $RESULTS"
echo "=== Done — $(date) ==="
