#[cfg(test)]
mod helper;
#[cfg(test)]
pub mod vm;

#[cfg(test)]
pub mod arch;
#[cfg(test)]
pub mod test;

#[cfg(test)]
mod tests {
    use crate::arch::*;
    use crate::test::*;
    use crate::vm::Vm;
    use anyhow::Result;
    use std::path::Path;

    fn test(mut vm: impl Vm) -> Result<bool> {
        let mut result = true;
        result &= strlen::all_tests(&mut vm)?;
        result &= strcat::all_tests(&mut vm)?;
        result &= cos::all_tests(&mut vm)?;
        result &= sin::all_tests(&mut vm)?;
        result &= rint::all_tests(&mut vm)?;
        result &= rintf::all_tests(&mut vm)?;
        Ok(result)
    }

    #[test]
    fn i486() -> Result<()> {
        //NOTE there is no i486 triple, just use the i586 instead
        let vm = x86::X86::new(
            "i586-linux-musl",
            Path::new(
                "/home/rbran/src/icicle-pingu/bins/i486-linux-musl-libc.so",
            ),
        )?;
        assert!(test(vm)?);
        Ok(())
    }

    #[test]
    fn i686() -> Result<()> {
        let vm = x86::X86::new(
            "i686-linux-musl",
            Path::new(
                "/home/rbran/src/icicle-pingu/bins/i686-linux-musl-libc.so",
            ),
        )?;
        assert!(test(vm)?);
        Ok(())
    }

    #[test]
    fn x86_64() -> Result<()> {
        let vm = x86_64::X86_64::new(Path::new(
            "/home/rbran/src/icicle-pingu/bins/x86_64-linux-musl-libc.so",
        ))?;
        assert!(test(vm)?);
        Ok(())
    }

    #[test]
    fn aarch64() -> Result<()> {
        let vm = aarch64::Aarch64::new(
            "aarch64-linux-musl",
            Path::new(
                "/home/rbran/src/icicle-pingu/bins/aarch64-linux-musl-libc.so",
            ),
        )?;
        assert!(test(vm)?);
        Ok(())
    }
}
