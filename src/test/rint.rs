use crate::vm::{Param, Return, Vm};
use anyhow::Result;

pub struct TestStatic {
    param: f64,
    result: f64,
}

impl TestStatic {
    fn test_on_vm(
        &self,
        fun_addr: u64,
        ret_addr: u64,
        vm: &mut impl Vm,
    ) -> Result<bool> {
        let mut params = [Param::F64(self.param)];
        let mut output = [Return::F64(0.0)];
        vm.call(fun_addr, ret_addr, &mut params, &mut output)?;
        let [Return::F64(output)] = output else { unreachable!() };
        Ok(output == self.result)
    }
}

pub const TESTS_STATIC: &[f64] =
    &[1.0, 0.0, 1.2, 8.4, 90.6, 1031.5, 2.5555555556, 90.00001, 1.0e-6, 1.0e+6];
pub fn all_tests(vm: &mut impl Vm) -> Result<bool> {
    const FN_SYM: &str = "rint";
    let fun_addr = vm.lookup_symbol(FN_SYM);
    let ret_addr = vm.lookup_symbol("_dlstart");

    let tests_static = TESTS_STATIC.into_iter().map(|value| TestStatic {
        param: *value,
        result: value.round(),
    });
    for (i, test) in tests_static.enumerate() {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            println!("{} Error test static {} f64({})", FN_SYM, i, test.param);
            return Ok(false);
        }
    }
    Ok(true)
}
