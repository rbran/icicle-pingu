use crate::vm::{IcicleHelper, Param, Return, Vm};
use anyhow::Result;
use icicle_mem::perm;

pub struct StrlenTestStatic {
    data: &'static [u8],
    result: u64,
}

impl StrlenTestStatic {
    fn test_on_vm(
        &self,
        fun_addr: u64,
        ret_addr: u64,
        vm: &mut impl Vm,
    ) -> Result<bool> {
        let mut params = [Param::HeapData(&self.data)];
        let mut output = [Return::Usize(0)];
        vm.call(fun_addr, ret_addr, &mut params, &mut output)?;
        let [Return::Usize(output)] = output else { unreachable!() };
        Ok(output == self.result)
    }
}

pub struct StrlenTestLong {
    data: u8,
    data_len: u64,
}

impl StrlenTestLong {
    fn test_on_vm(
        &self,
        fun_addr: u64,
        ret_addr: u64,
        vm: &mut impl Vm,
    ) -> Result<bool> {
        let write_str = |vm: &mut IcicleHelper| {
            // TODO improve that
            let addr = vm.malloc(self.data_len + 1)?;
            vm.icicle.cpu.mem.write_bytes(
                addr,
                &vec![self.data; self.data_len.try_into().unwrap()],
                perm::NONE,
            )?;
            vm.icicle.cpu.mem.write_u8(
                addr + self.data_len as u64,
                0,
                perm::NONE,
            )?;
            Ok(addr)
        };
        let mut params = [Param::HeapFn(Box::new(write_str))];
        let mut output = [Return::Usize(0)];
        vm.call(fun_addr, ret_addr, &mut params, &mut output)?;
        let [Return::Usize(output)] = output else { unreachable!() };
        Ok(output == self.data_len)
    }
}

pub const FUNNY_STRING: &str =
    "aç😂¢ŴƉǁǆǗǱȌȘȤȮȵȸḐṑẜẞẟểỻɖʭʺ   ̉ͶἢЉՃ٣דܣޓޓਦଖሶᓅ᠊ᡈ†‖⁷₧℧Ⅷ↷∧⍗␖Ⓜ┳▐▮♁🭂✺Ꮅࠕࡕ\x00";
pub const TESTS_STATIC: [(&[u8], u64); 10] = [
    (b"test\x00", 4),
    (b"test\x00123", 4),
    (b"\x00", 0),
    (b"\x00test", 0),
    (b"test\xff\x00test", 5),
    (b"\x01\x02\x03\x04\r\n\x7F\x00", 7),
    (b"\xff\xfe\xfd\xfc\xa0\xa1\xa2\xa3\x00", 8),
    (b"\xff\x00\x21\x00", 1),
    (b"\x00\x20\x00\x01\x00", 0),
    (FUNNY_STRING.as_bytes(), FUNNY_STRING.len() as u64 - 1), //-1 for \x00
];
pub const TESTS_LONG: [(u8, u64); 2] = [(0x01, 0x1234), (0xff, 0x4321)];
pub fn all_tests(vm: &mut impl Vm) -> Result<bool> {
    const FN_SYM: &str = "strlen";
    let fun_addr = vm.lookup_symbol(FN_SYM);
    let ret_addr = vm.lookup_symbol("_dlstart");

    // test short strings
    let tests_static = TESTS_STATIC
        .into_iter()
        .map(|(data, result)| StrlenTestStatic { data, result });
    for (i, test) in tests_static.enumerate() {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            println!("{} Error test static {} ", FN_SYM, i);
            return Ok(false);
        }
    }

    // test long strings
    let tests_long = TESTS_LONG
        .into_iter()
        .map(|(data, data_len)| StrlenTestLong { data, data_len });
    for (i, test) in tests_long.enumerate() {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            println!("{} Error test long {} ", FN_SYM, i);
            return Ok(false);
        }
    }
    Ok(true)
}
