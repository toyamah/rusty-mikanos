#!/usr/bin/env bash

set -eu
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd -P)

cd $script_dir
mkdir -p usb/classdriver
mkdir -p usb/xhci
mkdir -p test

cd "$script_dir"/../../official/kernel
files=$(\find . -maxdepth 3 \
  -name "*.cpp" \
  -o -name "*.hpp" \
  -o -name "*.c" \
  -o -name "*.h" \
  -o -name "*.asm" \
  | sed -e 's/\.\///g')

parse_params() {
  clear=0

  while :; do
    case "${1-}" in
    -v | --verbose) set -x ;;
    -c | --clear) clear=1 ;;
    -?*) die "Unknown option: $1" ;;
    *) break ;;
    esac
    shift
  done

  args=("$@")

  return 0
}

make_links() {
  cd "$script_dir"

  for file in $files; do
    if [ -e "$file" -a ! -L "$file" ]; then
      continue
    fi

    depth=$(\echo "$file" | grep -o "/" | wc -l)
    path="/"
    for ((i=0; i < $depth; i++)); do
      path="$path../"
    done

    ln -s ../.."$path"official/kernel/"$file" "$file"
  done
}

clear() {
  cd "$script_dir"

  for file in $files; do
    if [ ! -L "$file" ]; then
      continue
    fi
    unlink "$file"
  done
}

parse_params "$@"

if [ $clear -eq 1 ]; then
  clear
else
  make_links
fi
