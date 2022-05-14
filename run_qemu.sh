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
  if [ $clippy -eq 1 ]; then
    cd kernel # run in only kernel because borrowed stdlib code in apps/shared_lib needs to be fixed.
    cargo clippy -- -Dwarnings
    cd -
  fi

  if [ $apps -eq 1 ]; then
    cargo build --release # build in release mode to optimize code
  else
    cd kernel
    cargo build --release # build in release mode to optimize code
    cd -
  fi
#  cargo build

  make -C apps/onlyhlt/ onlyhlt

  MIKANOS_DIR=$PWD $HOME/osbook/devenv/run_mikanos.sh
}

build_and_run_official() {
  official_dir="$script_dir"/official

#  export RUSTFLAGS="-C link-arg=$LDFLAGS"
#  for cargo_manifest in $(ls apps/*/Cargo.toml)
#  do
#    app_dir=$(dirname $cargo_manifest)
#    if [ $app_dir == "apps/shared_lib" ]; then
#      continue
#    fi
#    app=$(basename $app_dir)
#    cd "${script_dir}/${app_dir}"
#    cargo build --release
#    cp ${app} $official_dir/apps/${app}/
#  done

  cd "$official_dir"
  ./build.sh run
}

parse_params() {
  official=0
  clippy=0
  apps=0

  while :; do
    case "${1-}" in
    -v | --verbose) set -x ;;
    -c | --clippy) clippy=1 ;;
    -a | --apps) apps=1 ;;
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
