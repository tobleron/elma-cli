#!/bin/bash
# Test GBNF Grammar Injection
# 
# This script tests that grammar injection is working correctly.

set -e

echo "==================================="
echo "GBNF Grammar Injection Test"
echo "==================================="
echo ""

# Test 1: Check grammar files exist
echo "Test 1: Checking grammar files..."
GRAMMAR_FILES=(
    "config/grammars/router_choice_1of5.json.gbnf"
    "config/grammars/speech_act_choice_1of3.json.gbnf"
    "config/grammars/mode_router_choice_1of4.json.gbnf"
    "config/grammars/complexity_choice_1of4.json.gbnf"
)

for file in "${GRAMMAR_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✓ $file exists"
    else
        echo "  ✗ $file MISSING"
        exit 1
    fi
done
echo ""

# Test 2: Check grammar mapping config
echo "Test 2: Checking grammar mapping..."
if [ -f "config/grammar_mapping.toml" ]; then
    echo "  ✓ grammar_mapping.toml exists"
    echo "  Contents:"
    cat config/grammar_mapping.toml | sed 's/^/    /'
else
    echo "  ✗ grammar_mapping.toml MISSING"
    exit 1
fi
echo ""

# Test 3: Build verification
echo "Test 3: Build verification..."
if cargo build --quiet 2>&1 | grep -q "error"; then
    echo "  ✗ Build FAILED"
    exit 1
else
    echo "  ✓ Build successful"
fi
echo ""

# Test 4: Run with grammar injection (manual test)
echo "Test 4: Manual grammar injection test"
echo "======================================"
echo ""
echo "To test grammar injection manually:"
echo ""
echo "1. Start elma-cli:"
echo "   cargo run -- --base-url http://192.168.1.186:8080"
echo ""
echo "2. Try classification requests:"
echo "   - 'classify: list files in current directory'"
echo "   - 'what mode: read Cargo.toml and summarize'"
echo ""
echo "3. Check trace logs for grammar injection:"
echo "   Look for: [GRAMMAR] injected grammar for profile=router"
echo "   Look for: [INTEL_GRAMMAR] injected grammar for unit=complexity_assessment"
echo ""
echo "4. Verify JSON output is valid:"
echo "   All router/speech_act/mode_router outputs should be 100% valid JSON"
echo ""

# Test 5: Check trace log location
echo "Test 5: Trace log location"
echo "=========================="
echo "Trace logs will be written to:"
echo "  sessions/<session_id>/trace_debug.log"
echo ""
echo "After running a session, check for grammar injection messages."
echo ""

echo "==================================="
echo "Grammar Injection Test Complete"
echo "==================================="
echo ""
echo "Next Steps:"
echo "1. Run elma-cli with a test request"
echo "2. Check trace logs for [GRAMMAR] messages"
echo "3. Verify 100% JSON validity in outputs"
echo ""
