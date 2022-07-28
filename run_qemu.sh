#!/usr/bin/env bash

#set -eu
set -e
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd -P)

build_and_run() {
  cd "$script_dir"

  # this allows rustc to find -lc and -lc++ libraries which are defined in x86~~-elf.json file
  # see https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags
  export RUSTFLAGS="-C link-arg=$LDFLAGS"

  # run clippy instead of CI because setting up the environment is bothersome.
  if [ $clippy -eq 1 ]; then
    cd kernel
    cargo clippy -- -Dwarnings
    cd -
    exit
  fi

  if [ $kernel -eq 1 ]; then
    cd kernel
    cargo build --release # build in release mode to optimize code
  fi

  cd "$script_dir"
  if [ $apps == "all" ]; then
    for cargo_manifest in $(ls apps/*/Cargo.toml); do
      app_dir=$(dirname $cargo_manifest)
      if [ $app_dir == "apps/shared_lib" ]; then
        continue
      fi
      cd "${script_dir}/${app_dir}"
      cargo build --release
    done
  elif [ ${#apps[@]} -gt 0 ]; then
    for app in ${apps[@]}; do
      cd "${script_dir}/apps/${app}"
      cargo build --release
    done
  fi

  cd $script_dir
  make -C apps/onlyhlt/ onlyhlt

  export APPS_DIR=apps
  export RESOURCE_DIR=resource
  MIKANOS_DIR=$PWD $HOME/osbook/devenv/run_mikanos.sh
}

build_and_run_official() {
  official_dir="$script_dir"/official

#  export RUSTFLAGS="-C link-arg=$LDFLAGS"
#  if [ ${#apps[@]} -gt 0 ]; then
#    for app in ${apps[@]}; do
#      cd "${script_dir}/apps/${app}"
#      cargo build --release
#      cp ${app} $official_dir/apps/${app}/
#    done
#  fi

  cd "$official_dir"
  export APPS_DIR=apps
  export RESOURCE_DIR=resource
  ./build.sh run
}

parse_params() {
  clippy=0
  kernel=0
  apps=()
  official=0

  while :; do
    case "${1-}" in
    -v | --verbose) set -x ;;
    -c | --clippy) clippy=1 ;;
    -k | --kernel) kernel=1 ;;
    --apps=*)
      if [[ "$1" =~ ^--apps= ]]; then
          apps_csv=$(echo $1 | sed -e 's/^--apps=//')
          IFS=',' read -ra apps <<< "$apps_csv"
      fi
    ;;
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
