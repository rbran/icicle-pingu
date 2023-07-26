use crate::vm::{Param, Return, Vm};
use anyhow::Result;

pub struct CosTestStatic {
    param: f64,
    result: f64,
}

impl CosTestStatic {
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
    &[1.0, 0.0, 1.2, 8.4, 90.0, 90.00001, 1.0e-6, 1.0e+6];
pub fn all_tests(vm: &mut impl Vm) -> Result<bool> {
    let fun_addr = vm.lookup_symbol("cos");
    let ret_addr = vm.lookup_symbol("_dlstart");

    let tests_static = TESTS_STATIC.into_iter().map(|value| CosTestStatic {
        param: *value,
        result: value.cos(),
    });
    for test in tests_static {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            return Ok(false);
        }
    }
    Ok(true)
}
