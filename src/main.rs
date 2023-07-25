use std::path::Path;

use anyhow::Result;

use strlen::StrlenTests;

mod strlen;

mod helper;

mod vm;

mod aarch64;
//mod arm;
mod x86;
mod x86_64;

fn main() -> Result<()> {
    let tests = strlen::all_tests();
    //NOTE there is no i486 triple, just use the i586 instead
    let mut i586_vm = x86::X86::new(
        "i586-linux-musl",
        Path::new("/home/rbran/src/icicle-pingu/bins/i486-linux-musl-libc.so"),
    )?;
    tests.clone().test_all(&mut i586_vm)?;

    let mut i686_vm = x86::X86::new(
        "i686-linux-musl",
        Path::new("/home/rbran/src/icicle-pingu/bins/i686-linux-musl-libc.so"),
    )?;
    tests.clone().test_all(&mut i686_vm)?;

    let mut x86_64_vm = x86_64::X86_64::new(Path::new(
        "/home/rbran/src/icicle-pingu/bins/x86_64-linux-musl-libc.so",
    ))?;
    tests.clone().test_all(&mut x86_64_vm)?;

    //let mut arm_vm = arm::ARM::new(
    //    "armv7-linux-musl",
    //    Path::new("/home/rbran/src/icicle-pingu/bins/arm-linux-musl-libc.so"),
    //)?;
    //tests.clone().test_all(&mut arm_vm)?;

    let mut aarch64_vm = aarch64::Aarch64::new(
        "aarch64-linux-musl",
        Path::new(
            "/home/rbran/src/icicle-pingu/bins/aarch64-linux-musl-libc.so",
        ),
    )?;
    tests.clone().test_all(&mut aarch64_vm)?;

    //let mut aarch64_be_vm = aarch64::Aarch64::new(
    //    "aarch64_be-linux-musl",
    //    Path::new("/home/rbran/src/icicle-pingu/bins/aarch64_be-linux-musl-libc.so"),
    //)?;
    //tests.clone().test_all(&mut aarch64_be_vm)?;

    Ok(())
}
