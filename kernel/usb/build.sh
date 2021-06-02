#!/usr/bin/env bash

set -eu
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd -P)

cd "$script_dir"/../../official/mikanos/kernel
files=$(\find . -maxdepth 3 -name "*.cpp" -o -name "*.hpp" -o -name "*.c"  | sed -e 's/\.\///g')
cd "$script_dir"

cd "$script_dir"
for file in $files; do
  depth=$(\echo "$file" | grep -o "/" | wc -l)
  path="/"
  for ((i=0; i < $depth; i++)); do
    path="$path../"
  done
  ln -f -s ../.."$path"official/mikanos/kernel/"$file" "$file"
done

source ~/osbook/devenv/buildenv.sh
make

#cd "$script_dir"
#for file in $files; do
#  unlink "$file"
#done
#make clean