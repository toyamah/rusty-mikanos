# rusty mikanos
A [MikanOS](https://github.com/uchan-nos/mikanos) implementation written in Rust.

## Setup
First, set up your environment following [the official setup steps](https://github.com/uchan-nos/mikanos-build/)

And then run these commands.
```shell
git submodule update --init --recursive
./kernel/usb/setup.sh
```

## Build and Launch
```shell
# build kernel and all applications then launch rusty mikanos using QEMU
./run_qemu.sh -k --apps=all

# build the official MikanOS and launch it if you want to check
./run_qemu.sh -o
```

## References
- [ゼロからのOS自作入門](https://zero.osdev.jp)
- [Writing an OS in Rust](https://os.phil-opp.com)
- [OSDev.org](https://wiki.osdev.org/Main_Page)
- [The Embedded Rust Book](https://docs.rust-embedded.org/book/intro/index.html)
- [Rust stdlib](https://github.com/rust-lang/rust)
- [x86_64 crate](https://github.com/rust-osdev/x86_64)