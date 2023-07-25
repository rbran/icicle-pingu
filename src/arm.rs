use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use icicle_mem::perm;
use icicle_vm;
use pcode::VarNode;

use crate::vm::{IcicleHelper, Param, Vm};

pub struct ARM {
    pub helper: IcicleHelper,
    r0: VarNode,
    sp: VarNode,
}

impl ARM {
    pub fn new(triple: &str, musl: &Path) -> Result<Self> {
        let mut vm = icicle_vm::build(&icicle_vm::cpu::Config {
            triple: triple.parse().unwrap(),
            enable_shadow_stack: false,
            ..icicle_vm::cpu::Config::default()
        })?;
        vm.env = icicle_vm::env::build_auto(&mut vm)?;
        vm.env
            .load(&mut vm.cpu, musl.as_os_str().as_bytes())
            .map_err(|e| anyhow!(e))?;

        let r0 = vm.cpu.arch.sleigh.get_reg("r0").unwrap().var;
        let sp = vm.cpu.arch.sleigh.get_reg("sp").unwrap().var;
        Ok(Self {
            helper: IcicleHelper::new(vm, 0x1000_0000, 0x1000_0000, 0x2000_0000, 0x1000_0000),
            r0,
            sp,
        })
    }

    fn stack_used(params: &[Param]) -> u64 {
        if params.len() > 1 {
            todo!();
        }
        0
    }

    fn set_stack(
        &mut self,
        _stack_pos: &mut u64,
        return_addr: u32,
        params: &mut [Param],
    ) -> Result<()> {
        for (idx, param) in params.iter_mut().enumerate() {
            match (idx, param) {
                (0, Param::Usize(value)) => self.helper.icicle.cpu.write_reg(self.r0, *value),
                (0, Param::HeapData(data)) => {
                    let addr = self.helper.malloc(data.len() as u64)?;
                    // write the heap
                    self.helper
                        .icicle
                        .cpu
                        .mem
                        .write_bytes(addr, data, perm::NONE)?;
                    // put the addr to the reg
                    self.helper.icicle.cpu.write_reg(self.r0, addr)
                }
                _ => todo!(),
            }
        }

        self.helper.icicle.cpu.write_reg(self.lr, return_addr);
        Ok(())
    }

    fn get_results(&mut self, results: &mut [Param]) -> Result<()> {
        match results {
            [] => {}
            [Param::Usize(value)] => *value = self.helper.icicle.cpu.read_reg(self.r0),
            _ => todo!(),
        }
        Ok(())
    }
}

impl Vm for ARM {
    fn helper(&self) -> &IcicleHelper {
        &self.helper
    }

    fn helper_mut(&mut self) -> &mut IcicleHelper {
        &mut self.helper
    }

    fn call(
        &mut self,
        function_addr: u64,
        return_addr: u64,
        params: &mut [Param],
        results: &mut [Param],
    ) -> Result<()> {
        //clean the heap
        self.helper.free_all();

        let stack_len = Self::stack_used(params);
        self.helper.set_stack_len(stack_len)?;

        let mut stack_pos = self.helper.stack_addr + self.helper.stack_size;
        self.set_stack(&mut stack_pos, return_addr as u32, params)?;
        // set stack addr to register
        self.helper.icicle.cpu.write_reg(self.sp, stack_pos);
        // set the function addr to pc
        self.helper.icicle.cpu.write_pc(function_addr);
        let vm_exit = self.helper.icicle.run_until(return_addr);
        if vm_exit != icicle_vm::VmExit::Breakpoint {
            bail!(
                "Vm exited at 0x{:08x} with {:?}",
                self.helper.icicle.cpu.read_pc(),
                vm_exit
            )
        }

        self.get_results(results)?;
        self.helper.icicle.cpu.reset();
        Ok(())
    }
}
