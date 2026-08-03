#!/bin/bash
set -euo pipefail

echo "=== Verifying TD1/TD2 Document Generation ==="

# Check formatting
echo "1. Running cargo fmt check..."
cargo fmt --all --check

echo "2. Running cargo clippy..."
cargo clippy --workspace --all-targets

echo "3. Running tests..."
cargo test --workspace

echo "4. Checking for Windows line endings..."
if grep -rl $'\r' crates/ --include="*.rs"; then
    echo "ERROR: Found Windows line endings!"
    exit 1
fi

echo "5. Checking for trailing whitespace..."
if grep -rl "[[:space:]]$" crates/ --include="*.rs"; then
    echo "ERROR: Found trailing whitespace!"
    exit 1
fi

echo "6. Checking mrz_line field usage..."
# Should only find in CLI for backward compatibility
CLI_MRZ_LINES=$(grep -r "mrz_line[12]" crates/synthpass-cli/src/ --include="*.rs" | wc -l)
OTHER_MRZ_LINES=$(grep -r "\.mrz_line[12]" crates/ --include="*.rs" | grep -v "synthpass-cli" | wc -l)

if [ "$OTHER_MRZ_LINES" -ne 0 ]; then
    echo "ERROR: Found direct mrz_line1/mrz_line2 field access outside CLI!"
    grep -r "\.mrz_line[12]" crates/ --include="*.rs" | grep -v "synthpass-cli"
    exit 1
fi

echo "7. Checking bench files use mrz_lines..."
BENCH_MRZ_LINES=$(grep -r "mrz_lines\[" crates/synthpass-bench/src/ --include="*.rs" | wc -l)
if [ "$BENCH_MRZ_LINES" -eq 0 ]; then
    echo "ERROR: Bench files not using mrz_lines array!"
    exit 1
fi

echo "8. Checking Labels struct..."
if ! grep -q "pub mrz_lines: Vec<String>" crates/synthpass-gen/src/labels.rs; then
    echo "ERROR: Labels struct not using mrz_lines: Vec<String>!"
    exit 1
fi

echo "9. Checking CLI backward compatibility..."
if ! grep -q "mrz_line3: Option<String>" crates/synthpass-cli/src/generate.rs; then
    echo "ERROR: CLI missing mrz_line3 field for TD1 support!"
    exit 1
fi

echo "✅ All checks passed! TD1/TD2 implementation is ready."
