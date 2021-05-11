#!/usr/bin/env bash
set -e
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd -P)

source ~/osbook/devenv/buildenv.sh

cd "$script_dir"/kernel
make clean
make
mv kernel.elf $script_dir/