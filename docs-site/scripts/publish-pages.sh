#!/usr/bin/env bash
# Publish the built docs to the organisation's GitHub Pages site, under /shadow/:
#
#   https://tiny-bubble-company.github.io/shadow/
#
# Used while this repository is private (GitHub Pages for private repositories needs a paid
# plan). The site repository holds only the generated HTML, never source. Once this repository
# is public, switch to .github/workflows/docs.yml: the URL stays the same.
#
#   docs-site/scripts/publish-pages.sh            # build and push
#   DRY_RUN=1 docs-site/scripts/publish-pages.sh  # build and stage, don't push
set -euo pipefail

PAGES_REPO="${PAGES_REPO:-Tiny-Bubble-Company/tiny-bubble-company.github.io}"
SITE="${SITE:-https://tiny-bubble-company.github.io}"
BASE="${BASE:-/shadow}"
here="$(cd "$(dirname "$0")/.." && pwd)"

cd "$here"
SITE="$SITE" BASE="$BASE" pnpm build

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
git clone --quiet --depth 1 "https://github.com/$PAGES_REPO.git" "$work/site"
cd "$work/site"

rm -rf "./${BASE#/}"
mkdir -p "./${BASE#/}"
cp -R "$here/dist/." "./${BASE#/}/"

# Crawlers read robots.txt only at the domain root, and llms.txt is looked up there too.
cp "$here/dist/robots.txt" robots.txt
cp "$here/dist/llms.txt" llms.txt
touch .nojekyll
if [ ! -f index.html ]; then
	cat > index.html <<EOF
<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Shadow — open-source privacy layer for AI</title>
<meta name="robots" content="noindex">
<link rel="canonical" href="$SITE$BASE/">
<meta http-equiv="refresh" content="0; url=$BASE/">
<a href="$BASE/">Shadow documentation</a>
</html>
EOF
fi

git add -A
if git diff --cached --quiet; then
	echo "Nothing changed."
	exit 0
fi
git -c user.name=kannancet -c user.email=kannancet@users.noreply.github.com \
	commit --quiet -m "Publish Shadow docs."
if [ -n "${DRY_RUN:-}" ]; then
	git show --stat --oneline HEAD | head -20
	echo "Dry run: not pushed."
else
	git push --quiet origin HEAD:main
	echo "Published to $SITE$BASE/"
fi
