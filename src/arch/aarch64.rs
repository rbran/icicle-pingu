use icicle_mem::perm;

use crate::vm::{IcicleHelper, Return};

use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use icicle_vm;
use pcode::VarNode;

use crate::vm::{Param, Vm};

pub struct Aarch64 {
    pub helper: IcicleHelper,
    regs: Vec<VarNode>,
    sp: VarNode,
}

impl Aarch64 {
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

        let regs = (0..=30)
            .map(|reg| {
                vm.cpu
                    .arch
                    .sleigh
                    .get_reg(&format!("x{}", reg))
                    .unwrap()
                    .var
            })
            .collect();
        let sp = vm.cpu.arch.sleigh.get_reg("sp").unwrap().var;
        Ok(Self {
            helper: IcicleHelper::new(
                vm,
                0x1000_0000,
                0x1000_0000,
                0x2000_0000,
                0x1000_0000,
            ),
            regs,
            sp,
        })
    }

    fn stack_used(params: &[Param]) -> u64 {
        if params.len() > 7 {
            todo!();
        }
        0
    }

    fn set_call(
        &mut self,
        _stack_pos: &mut u64,
        return_addr: u64,
        params: &mut [Param],
    ) -> Result<()> {
        if params.len() > 7 {
            todo!()
        }
        for (i, param) in params.iter_mut().enumerate() {
            let reg = self.regs[i];
            match param {
                Param::Usize(value) => {
                    self.helper.icicle.cpu.write_reg(reg, *value)
                }
                Param::HeapData(data) => {
                    let addr = self.helper.malloc(data.len() as u64)?;
                    // write the heap
                    self.helper.icicle.cpu.mem.write_bytes(
                        addr,
                        data,
                        perm::NONE,
                    )?;
                    // put the addr to the reg
                    self.helper.icicle.cpu.write_reg(reg, addr)
                }
                Param::HeapFn(write_data) => {
                    let addr = write_data(&mut self.helper)?;
                    // put the addr to the reg
                    self.helper.icicle.cpu.write_reg(reg, addr)
                }
            }
        }

        // write the return addr to x30/LR
        self.helper.icicle.cpu.write_reg(self.regs[30], return_addr);
        Ok(())
    }

    fn get_results(&mut self, results: &mut [Return]) -> Result<()> {
        if results.len() > 7 {
            todo!()
        }
        for (i, result) in results.iter_mut().enumerate() {
            let reg = self.regs[i];
            match result {
                Return::Usize(value) => {
                    *value = self.helper.icicle.cpu.read_reg(reg)
                }
                Return::CString(data) => {
                    let addr = self.helper.icicle.cpu.read_reg(reg);
                    self.helper.icicle.cpu.mem.read_cstr(addr, data)?;
                }
            }
        }
        Ok(())
    }
}

impl Vm for Aarch64 {
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
        results: &mut [Return],
    ) -> Result<()> {
        //clean the heap
        self.helper.free_all();

        let stack_len = Self::stack_used(params);
        self.helper.set_stack_len(stack_len)?;

        let mut stack_pos = self.helper.stack_addr + self.helper.stack_size;
        self.set_call(&mut stack_pos, return_addr, params)?;
        // set stack addr to register
        self.helper.icicle.cpu.write_reg(self.sp, stack_pos);
        // set the function addr to pc
        self.helper.icicle.cpu.write_pc(function_addr);

        let vm_exit = self.helper.icicle.run_until(return_addr);
        if vm_exit != icicle_vm::VmExit::Breakpoint {
            bail!(
                "Vm exited at 0x{:016x} with {:?}",
                self.helper.icicle.cpu.read_pc(),
                vm_exit
            )
        }

        self.get_results(results)?;
        self.helper.icicle.cpu.reset();
        Ok(())
    }
}
