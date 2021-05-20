References
- https://doc.rust-lang.org/rustc/codegen-options/index.html
- https://docs.rust-embedded.org/embedonomicon/custom-target.html

### llvm-target
see https://clang.llvm.org/docs/CrossCompilation.html#target-triple
> The triple has the general format <arch><sub>-<vendor>-<sys>-<abi>

Set x86_64-unknown-elf
- arch : x86_64
- vendor : unknown because of unclear
- sys : none because of running on bare metal
  - `The system name is generally the OS (linux, darwin), but could be special like the bare-metal “none”.`
- abi: elf 

see also https://os.phil-opp.com/minimal-rust-kernel/#target-specification


### panic-strategy
see https://docs.rust-embedded.org/embedonomicon/custom-target.html#fill-the-target-file
> Decide on a panicking strategy.
> A bare metal implementation will likely use "panic-strategy": "abort".
> If you decide not to abort on panicking, unless you tell Cargo to per-project, you must define an eh_personality function.

see also https://os.phil-opp.com/minimal-rust-kernel/#target-specification

### linker, linker-flavor, post-link-target
see https://docs.rust-embedded.org/embedonomicon/custom-target.html#fill-the-target-file

> Change the linker if integrating with an existing toolchain.
> For example, if you're using a toolchain that uses a custom build of gcc, set "linker-flavor": "gcc" and linker to the command name of your linker.
> If you require additional linker arguments, use pre-link-args and post-link-args

