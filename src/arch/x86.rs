use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use icicle_mem::perm;
use icicle_vm;
use pcode::VarNode;

use crate::vm::{IcicleHelper, Param, Return, Vm};

pub struct X86 {
    pub helper: IcicleHelper,
    eax: VarNode,
    esp: VarNode,
}

impl X86 {
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

        let eax = vm.cpu.arch.sleigh.get_reg("EAX").unwrap().var;
        let esp = vm.cpu.arch.sleigh.get_reg("ESP").unwrap().var;
        Ok(Self {
            helper: IcicleHelper::new(
                vm,
                0x1000_0000,
                0x1000_0000,
                0x2000_0000,
                0x1000_0000,
            ),
            eax,
            esp,
        })
    }

    fn stack_used(params: &[Param]) -> u64 {
        params
            .iter()
            .map(|param| match param {
                Param::Usize(_) => 4,
                Param::HeapData(_) => 4,
                Param::HeapFn(_) => 4,
            })
            .sum::<u64>()
            // + 4 for the return address added to the stack
            + 4
    }

    fn set_call(
        &mut self,
        return_addr: u32,
        params: &mut [Param],
    ) -> Result<u64> {
        //TODO min len for the stack
        let stack_len = Self::stack_used(params).max(0x1000);
        self.helper.set_stack_len(stack_len)?;

        let mut stack_pos = self.helper.stack_addr + self.helper.stack_size;

        for param in params.into_iter().rev() {
            match param {
                Param::Usize(value) => {
                    stack_pos -= 4;
                    self.helper.icicle.cpu.mem.write_u32(
                        stack_pos,
                        *value as u32,
                        perm::NONE,
                    )?;
                }
                Param::HeapData(data) => {
                    let addr = self.helper.malloc(data.len() as u64)?;
                    self.helper.icicle.cpu.mem.write_bytes(
                        addr,
                        data,
                        perm::NONE,
                    )?;
                    stack_pos -= 4;
                    self.helper.icicle.cpu.mem.write_u32(
                        stack_pos,
                        addr as u32,
                        perm::NONE,
                    )?;
                }
                Param::HeapFn(write_data) => {
                    let addr = write_data(&mut self.helper)?;
                    stack_pos -= 4;
                    self.helper.icicle.cpu.mem.write_u32(
                        stack_pos,
                        addr as u32,
                        perm::NONE,
                    )?;
                }
            }
        }

        // add the return addr to the stack
        stack_pos -= 4;
        self.helper.icicle.cpu.mem.write_u32(
            stack_pos,
            return_addr,
            perm::NONE,
        )?;
        Ok(stack_pos)
    }

    fn get_results(&mut self, results: &mut [Return]) -> Result<()> {
        let result = match results {
            [] => return Ok(()),
            [result] => result,
            _ => todo!(),
        };
        match result {
            Return::Usize(value) => {
                *value = self.helper.icicle.cpu.read_reg(self.eax)
            }
            Return::CString(data) => {
                let addr = self.helper.icicle.cpu.read_reg(self.eax);
                self.helper.icicle.cpu.mem.read_cstr(addr, data)?;
            }
        }
        Ok(())
    }
}

impl Vm for X86 {
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

        let stack_addr = self.set_call(return_addr as u32, params)?;
        // set stack addr to register
        self.helper.icicle.cpu.write_reg(self.esp, stack_addr);

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
