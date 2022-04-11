#!/usr/bin/env bash

#set -eu
set -e
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd -P)

build_and_run() {
  cd "$script_dir"

  # this allows rustc find -lc and -lc++ libraries which are defined in x86~~-elf.json file
  # see https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags
  export RUSTFLAGS="-C link-arg=$LDFLAGS"

  # run clippy instead of run on Github Actions because setting up the environment is bothersome.
#  cargo clippy -- -Dwarnings

  cargo build --release # build in release mode to optimize code
#  cargo build

  cd apps/rpn
  cargo build --release
  cd -

  make -C apps/onlyhlt/ onlyhlt

  MIKANOS_DIR=$PWD $HOME/osbook/devenv/run_mikanos.sh
}

build_and_run_official() {
  official_dir="$script_dir"/official

#  cd "$script_dir"
#  export RUSTFLAGS="-C link-arg=$LDFLAGS"
#  cd apps
#  cargo build --bin aa
#  cp rpn/rpn $official_dir/apps/rpn

  cd "$official_dir"
  ./build.sh run
}

parse_params() {
  official=0

  while :; do
    case "${1-}" in
    -v | --verbose) set -x ;;
    -o | --official) official=1 ;;
    -?*) die "Unknown option: $1" ;;
    *) break ;;
    esac
    shift
  done

  args=("$@")

  return 0
}

parse_params "$@"

source ~/osbook/devenv/buildenv.sh

if [ $official -eq 1 ]; then
  build_and_run_official
else
  build_and_run
fi
