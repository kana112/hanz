#!/bin/sh

set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 VERSION" >&2
    exit 2
fi

tag=$1
product_name=hanz
release="$product_name-$tag-arm64-darwin"
root_dir=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
dist_dir="$root_dir/dist"
release_dir="$dist_dir/$release"

cargo build --release --manifest-path "$root_dir/Cargo.toml"

mkdir -p "$release_dir"
cp "$root_dir/LICENSE" "$root_dir/README.md" "$root_dir/target/release/$product_name" "$release_dir/"
tar cvfz "$dist_dir/$release.tar.gz" -C "$dist_dir" "$release"
