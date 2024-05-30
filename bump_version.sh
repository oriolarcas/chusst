#!/bin/bash

usage() {
    echo "Usage: $0 <major> <minor> <patch>"
    echo "Example for 0.1.56: '$0 0 1 56'"
    exit 1
}

num_re='^(0|[1-9][0-9]*)$'

if [[ ! "$1" =~ $num_re ]] || [[ ! "$2" =~ $num_re ]] || [[ ! "$3" =~ $num_re ]]; then
    usage
fi

new_ver="$1.$2.$3"

cd -- "$(dirname "$0")"

echo "Bumping version to $new_ver"

echo Updating package.json
npm version --allow-same-version --commit-hooks false --git-tag-version false "$new_ver"

echo Updating Cargo.toml
cargo set-version -p chusst-gen "$new_ver"

echo Updating tauri.conf.json
sed -E -i'' "s/\"version\": \"[0-9.]+\"/\"version\": \"$new_ver\"/" src-tauri/tauri.conf.json
