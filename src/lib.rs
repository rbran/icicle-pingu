use std::path::Path;

use anyhow::Result;

mod helper;
pub mod vm;

pub mod arch;
pub mod test;

use arch::*;
use test::*;
use vm::Vm;

fn test(mut vm: impl Vm) -> Result<()> {
    let strlen = strlen::all_tests(&mut vm)?;
    println!("strlen {}", strlen);
    assert!(strlen);
    let strcat = strcat::all_tests(&mut vm)?;
    println!("strcat {}", strcat);
    assert!(strcat);
    Ok(())
}

#[test]
fn i486() -> Result<()> {
    //NOTE there is no i486 triple, just use the i586 instead
    let vm = x86::X86::new(
        "i586-linux-musl",
        Path::new("/home/rbran/src/icicle-pingu/bins/i486-linux-musl-libc.so"),
    )?;
    test(vm)
}

#[test]
fn i686() -> Result<()> {
    let vm = x86::X86::new(
        "i686-linux-musl",
        Path::new("/home/rbran/src/icicle-pingu/bins/i686-linux-musl-libc.so"),
    )?;
    test(vm)
}

#[test]
fn x86_64() -> Result<()> {
    let vm = x86_64::X86_64::new(Path::new(
        "/home/rbran/src/icicle-pingu/bins/x86_64-linux-musl-libc.so",
    ))?;
    test(vm)
}

#[test]
fn aarch64() -> Result<()> {
    let vm = aarch64::Aarch64::new(
        "aarch64-linux-musl",
        Path::new(
            "/home/rbran/src/icicle-pingu/bins/aarch64-linux-musl-libc.so",
        ),
    )?;
    test(vm)
}