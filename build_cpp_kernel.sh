#!/usr/bin/env bash
set -e

clang++ -O2 -Wall -g --target=x86_64-elf -ffreestanding -mno-red-zone \
-fno-exceptions -fno-rtti -std=c++17 -c kernel/main.cpp

ld.lld --entry KernelMain -z norelro --image-base 0x100000 --static \
-o kernel.elf main.o
