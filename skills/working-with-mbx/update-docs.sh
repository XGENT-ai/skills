#!/usr/bin/env bash
set -euo pipefail

# Update mbx (Mr. Boxington) documentation
# This script fetches the latest docs from mr-boxington.jdx.dev and converts them to markdown

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REFS_DIR="${SCRIPT_DIR}/references"
TEMP_OUTPUT="${SCRIPT_DIR}/.temp-output"

echo "=== Updating mbx Documentation ==="

# Clean up previous temp output
rm -rf "${TEMP_OUTPUT}"

# Fetch the whole doc set from mr-boxington.jdx.dev.
# The site is small (~50 pages) and publishes a complete sitemap.xml, so sitemap
# discovery is used instead of --skip-sitemap: it is faster and finds every page.
echo "Fetching docs from mr-boxington.jdx.dev..."
npx @mdream/crawl \
  "https://mr-boxington.jdx.dev/**" \
  --output "${TEMP_OUTPUT}" \
  --artifacts "markdown"

if [ ! -d "${TEMP_OUTPUT}" ]; then
  echo "Crawl produced no output" >&2
  exit 1
fi

# Mirror the crawled tree into references/ so reference paths match doc URLs:
#   https://mr-boxington.jdx.dev/cli/explain -> references/cli/explain.md
echo "Copying documentation to references/..."
rm -rf "${REFS_DIR}"
mkdir -p "${REFS_DIR}"
cp -R "${TEMP_OUTPUT}/." "${REFS_DIR}/"

# Clean up temp output
rm -rf "${TEMP_OUTPUT}"

echo "=== Update complete ==="
echo "Documentation updated in: ${REFS_DIR}"
echo ""
echo "Pages: $(find "${REFS_DIR}" -name "*.md" | wc -l | tr -d ' ')"
for dir in "${REFS_DIR}"/*/; do
  if [ -d "$dir" ]; then
    count=$(find "$dir" -name "*.md" 2>/dev/null | wc -l | tr -d ' ')
    echo "  $(basename "$dir")/: $count files"
  fi
done
