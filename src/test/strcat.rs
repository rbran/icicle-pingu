use crate::{
    test::strlen,
    vm::{IcicleHelper, Param, Return, Vm},
};
use anyhow::Result;
use icicle_mem::perm;

pub struct StrcatTestStatic {
    src: &'static [u8],
    dst: &'static [u8],
    result: &'static [u8],
}

impl StrcatTestStatic {
    fn test_on_vm(
        &self,
        fun_addr: u64,
        ret_addr: u64,
        vm: &mut impl Vm,
    ) -> Result<bool> {
        let mut params =
            [Param::HeapData(&self.src), Param::HeapData(&self.dst)];
        let mut output =
            [Return::CString(Vec::with_capacity(self.result.len()))];
        vm.call(fun_addr, ret_addr, &mut params, &mut output)?;
        let [Return::CString(output)] = output else { unreachable!() };
        Ok(output == self.result)
    }
}

pub struct StrcatTestLong {
    src: (u8, u64),
    dst: (u8, u64),
    res: (u8, u64, u8, u64),
}

impl StrcatTestLong {
    fn test_on_vm(
        &self,
        fun_addr: u64,
        ret_addr: u64,
        vm: &mut impl Vm,
    ) -> Result<bool> {
        let write_str = |data: u8, len: u64| {
            move |vm: &mut IcicleHelper| {
                // TODO improve that
                let src_addr = vm.malloc(len + 1)?;
                for i in 0..len {
                    vm.icicle.cpu.mem.write_u8(
                        src_addr + i,
                        data,
                        perm::NONE,
                    )?;
                }
                vm.icicle.cpu.mem.write_u8(src_addr + len, 0, perm::NONE)?;
                Ok(src_addr)
            }
        };
        let mut params = [
            Param::HeapFn(Box::new(write_str(self.src.0, self.src.1))),
            Param::HeapFn(Box::new(write_str(self.dst.0, self.dst.1))),
        ];
        let mut output = [Return::CString(Vec::with_capacity(
            (self.res.1 + self.res.3).try_into().unwrap(),
        ))];
        vm.call(fun_addr, ret_addr, &mut params, &mut output)?;
        let [Return::CString(output)] = output else { unreachable!() };
        let result = (0..self.res.1)
            .map(|_| self.res.0)
            .chain((0..self.res.3).map(|_| self.res.2));
        Ok(output.into_iter().zip(result).all(|(x, y)| x == y))
    }
}

pub const TESTS_STATIC: [(&[u8], &[u8], &[u8]); 2] = [
    (b"\x01\x02\x03\x00", b"\x00\x01\x02\x03", b"\x01\x02\x03"),
    (b"\x00\x02\x03\x00", b"\x01\x02\x03\x00", b"\x01\x02\x03"),
];

const TESTS_LONG: [((u8, u64), (u8, u64), (u8, u64, u8, u64)); 2] = [
    ((0x01, 0x1234), (0x02, 0x4321), (0x01, 0x1234, 0x02, 0x4321)),
    ((0xff, 0x1000), (0xfe, 0x1000), (0xff, 0x1000, 0xfe, 0x1000)),
];

pub fn all_tests(vm: &mut impl Vm) -> Result<bool> {
    let fun_addr = vm.lookup_symbol("strcat");
    let ret_addr = vm.lookup_symbol("_dlstart");

    // test strlen tests with an empty string
    let tests_static =
        strlen::TESTS_STATIC
            .into_iter()
            .map(|(src, _len)| StrcatTestStatic {
                src,
                dst: b"\x00",
                // NOTE result don't include the \x00
                result: &src[0..src.iter().position(|x| *x == 0).unwrap()],
            });
    for test in tests_static {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            return Ok(false);
        }
    }

    // test short strings
    let tests_static = TESTS_STATIC
        .into_iter()
        .map(|(src, dst, result)| StrcatTestStatic { src, dst, result });
    for test in tests_static {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            return Ok(false);
        }
    }

    // test long strings
    let tests_long = TESTS_LONG
        .into_iter()
        .map(|(src, dst, res)| StrcatTestLong { src, dst, res });

    for test in tests_long {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            return Ok(false);
        }
    }
    Ok(true)
}
