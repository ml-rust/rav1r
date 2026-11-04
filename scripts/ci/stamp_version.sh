#!/usr/bin/env bash
# Stamp the crate version from a release tag.
#
#   scripts/ci/stamp_version.sh <version>       # e.g. 0.1.0, 0.1.0-beta.2
#
# Rewrites `[package] version` in Cargo.toml.
#
# Only a prerelease tag changes anything. release-validate.yml has already
# checked that the tag's base version matches the manifest. For `v0.1.0` this
# does nothing, and for `v0.1.0-beta.2` it appends the suffix.

set -euo pipefail

VERSION="${1:?usage: stamp_version.sh <version>}"

CURRENT=$(cargo metadata --no-deps --format-version=1 \
    | jq -r '.packages[] | select(.name == "rav1r") | .version')

if [[ "$VERSION" == "$CURRENT" ]]; then
    echo "Version already $VERSION, nothing to stamp."
    exit 0
fi

# The first `version = "..."` at the start of a line is [package]. Every
# dependency version in the manifest is an inline-table or same-line form.
perl -i -pe 'if (!$done && /^version = "/) { s/^version = ".*"/version = "'"$VERSION"'"/; $done=1 }' Cargo.toml

echo "Stamped rav1r: $CURRENT -> $VERSION"
