use crate::vm::{Param, Return, Vm};
use anyhow::Result;

pub struct TestStatic {
    param: String,
    result: i64,
}

impl TestStatic {
    fn test_on_vm(
        &self,
        fun_addr: u64,
        ret_addr: u64,
        vm: &mut impl Vm,
    ) -> Result<bool> {
        let mut params = [Param::HeapData(self.param.as_bytes())];
        let mut output = [Return::I64(0)];
        vm.call(fun_addr, ret_addr, &mut params, &mut output)?;
        let [Return::I64(output)] = output else { unreachable!() };
        Ok(output == self.result)
    }
}

pub const TESTS_STATIC: &[&str] = &[
    "0",
    "1",
    "9",
    "10",
    "9876",
    "1337",
    "0000000000000000000",
    "9223372036854775807",
    "-1",
    "-0",
    "-9223372036854775807",
    "-9223372036854775808",
];
pub fn all_tests(vm: &mut impl Vm) -> Result<bool> {
    const FN_SYM: &str = "atoll";
    let fun_addr = vm.lookup_symbol(FN_SYM);
    let ret_addr = vm.lookup_symbol("_dlstart");

    let tests_static = TESTS_STATIC.into_iter().map(|value| TestStatic {
        param: format!("{}\x00", value),
        result: i64::from_str_radix(value, 10).unwrap(),
    });
    for (i, test) in tests_static.enumerate() {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            println!("{} Error test static {} i64({})", FN_SYM, i, test.result);
            return Ok(false);
        }
    }
    Ok(true)
}
