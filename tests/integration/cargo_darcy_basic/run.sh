#!/bin/sh
set -eu

root_dir=$(cd "$(dirname "$0")" && pwd)
cd "$root_dir/app"

out=$(cargo run --quiet)

echo "$out"

expected="darcy_main=90 darcy_calc=90 rust=90"
if [ "$out" != "$expected" ]; then
  echo "unexpected output" >&2
  exit 1
fi
