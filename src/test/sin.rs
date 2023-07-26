use super::cos::TESTS_STATIC;
use crate::vm::{Param, Return, Vm};
use anyhow::Result;

pub struct SinTestStatic {
    param: f64,
    result: f64,
}

impl SinTestStatic {
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

pub fn all_tests(vm: &mut impl Vm) -> Result<bool> {
    let fun_addr = vm.lookup_symbol("sin");
    let ret_addr = vm.lookup_symbol("_dlstart");

    let tests_static = TESTS_STATIC.into_iter().map(|value| SinTestStatic {
        param: *value,
        result: value.sin(),
    });
    for test in tests_static {
        if !test.test_on_vm(fun_addr, ret_addr, vm)? {
            return Ok(false);
        }
    }
    Ok(true)
}
