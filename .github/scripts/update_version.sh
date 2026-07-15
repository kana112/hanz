#!/bin/sh

set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 VERSION" >&2
    exit 2
fi

to_version=$1
case "$to_version" in
    ''|*[!0-9A-Za-z.-]*)
        echo "invalid version: $to_version" >&2
        exit 2
        ;;
esac

template=.github/templates/README.md
if [ ! -f "$template" ]; then
    echo "README template not found: $template" >&2
    exit 1
fi

cargo_tmp=$(mktemp "${TMPDIR:-/tmp}/hanz-cargo.XXXXXX")
readme_tmp=$(mktemp "${TMPDIR:-/tmp}/hanz-readme.XXXXXX")
trap 'rm -f "$cargo_tmp" "$readme_tmp"' EXIT HUP INT TERM

sed "s/^version = \".*\"/version = \"$to_version\"/" Cargo.toml >"$cargo_tmp"
sed "s/\${VERSION}/$to_version/g" "$template" >"$readme_tmp"

mv "$cargo_tmp" Cargo.toml
mv "$readme_tmp" README.md

trap - EXIT HUP INT TERM
