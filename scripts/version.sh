#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: version.sh <version>}"

if [ "$(grep -c '^version = ' Cargo.toml)" != "1" ]; then
  echo "version.sh : Cargo.toml ne contient pas exactement une ligne 'version = '" >&2
  exit 1
fi

sed -i.bak "s/^version = .*/version = \"$version\"/" Cargo.toml
rm -f Cargo.toml.bak

grep -q "^version = \"$version\"$" Cargo.toml

cargo metadata --format-version 1 > /dev/null

grep -A1 '^name = "equides-api"$' Cargo.lock | grep -q "^version = \"$version\"$"

echo "version.sh : Cargo.toml et Cargo.lock passés en $version"
