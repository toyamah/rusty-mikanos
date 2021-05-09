#!/usr/bin/env bash

#set -eu
set -e
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd -P)

cd ~/edk2
source edksetup.sh
build

$HOME/osbook/devenv/run_qemu.sh \
  ./Build/MikanLoaderX64/DEBUG_CLANG38/X64/Loader.efi \
  "$script_dir"/kernel.elf