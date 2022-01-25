# kernel/lib
This directory contains code that is used by the kernel cargo.  
The purpose of the directory is to enable to test code on a local machine independently.

## Run tests
```shell
cargo test --target x86_64-unknown-linux-gnu -Z build-std
```