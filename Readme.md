## Tester for the [Icicle-emu](https://github.com/icicle-emu/icicle-emu) project

This repo contains multiple [musl](https://musl.libc.org/) binaries in `bin`,
each compiled in a diferent architecture using
[musl-cross-make](https://github.com/richfelker/musl-cross-make).

The objective is to implement tests using the musl binary to find errors in the
icicle execution, or prove it's correctness.
