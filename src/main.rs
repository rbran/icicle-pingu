use std::path::Path;

use anyhow::Result;

mod helper;
mod vm;

mod test;
mod arch;

use test::*;
use arch::*;

fn main() -> Result<()> {
    //NOTE there is no i486 triple, just use the i586 instead
    let mut i586_vm = x86::X86::new(
        "i586-linux-musl",
        Path::new("/home/rbran/src/icicle-pingu/bins/i486-linux-musl-libc.so"),
    )?;
    assert!(strlen::all_tests(&mut i586_vm)?);
    assert!(strcat::all_tests(&mut i586_vm)?);

    let mut i686_vm = x86::X86::new(
        "i686-linux-musl",
        Path::new("/home/rbran/src/icicle-pingu/bins/i686-linux-musl-libc.so"),
    )?;
    assert!(strlen::all_tests(&mut i686_vm)?);
    assert!(strcat::all_tests(&mut i686_vm)?);

    let mut x86_64_vm = x86_64::X86_64::new(Path::new(
        "/home/rbran/src/icicle-pingu/bins/x86_64-linux-musl-libc.so",
    ))?;
    assert!(strlen::all_tests(&mut x86_64_vm)?);
    assert!(strcat::all_tests(&mut x86_64_vm)?);

    let mut aarch64_vm = aarch64::Aarch64::new(
        "aarch64-linux-musl",
        Path::new(
            "/home/rbran/src/icicle-pingu/bins/aarch64-linux-musl-libc.so",
        ),
    )?;
    assert!(strlen::all_tests(&mut aarch64_vm)?);
    assert!(strcat::all_tests(&mut aarch64_vm)?);

    Ok(())
}
