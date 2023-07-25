use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use icicle_mem::perm;
use icicle_vm;
use pcode::VarNode;

use crate::vm::{IcicleHelper, Param, Vm};

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
                Param::StackData(data) => data.len() as u64,
                Param::HeapFn(_, _) => 4,
                Param::StackFn(len, _) => *len,
            })
            .sum::<u64>()
            // + 4 for the return address added to the stack
            + 4
    }

    fn set_stack(
        &mut self,
        stack_pos: &mut u64,
        return_addr: u32,
        params: &mut [Param],
    ) -> Result<()> {
        for param in params {
            match param {
                Param::Usize(value) => {
                    *stack_pos -= 4;
                    self.helper.icicle.cpu.mem.write_u32(
                        *stack_pos,
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
                    *stack_pos -= 4;
                    self.helper.icicle.cpu.mem.write_u32(
                        *stack_pos,
                        addr as u32,
                        perm::NONE,
                    )?;
                }
                Param::HeapFn(len, write_data) => {
                    *stack_pos -= 4;
                    let addr = self.helper.malloc(*len)?;
                    write_data(&mut self.helper.icicle.cpu.mem, addr)?;
                    self.helper.icicle.cpu.mem.write_u32(
                        *stack_pos,
                        addr as u32,
                        perm::NONE,
                    )?;
                }
                Param::StackData(data) => {
                    *stack_pos -= data.len() as u64;
                    self.helper.icicle.cpu.mem.write_bytes(
                        *stack_pos,
                        data,
                        perm::NONE,
                    )?;
                }
                Param::StackFn(len, write_data) => {
                    *stack_pos -= *len;
                    write_data(&mut self.helper.icicle.cpu.mem, *stack_pos)?;
                }
            }
        }

        // add the return addr to the stack
        *stack_pos -= 4;
        self.helper.icicle.cpu.mem.write_u32(
            *stack_pos,
            return_addr,
            perm::NONE,
        )?;
        Ok(())
    }

    fn get_results(&mut self, results: &mut [Param]) -> Result<()> {
        match results {
            [] => {}
            [Param::Usize(value)] => {
                *value = self.helper.icicle.cpu.read_reg(self.eax)
            }
            _ => todo!(),
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
        results: &mut [Param],
    ) -> Result<()> {
        //clean the heap
        self.helper.free_all();

        let stack_len = Self::stack_used(params);
        self.helper.set_stack_len(stack_len)?;

        let mut stack_pos = self.helper.stack_addr + self.helper.stack_size;
        self.set_stack(&mut stack_pos, return_addr as u32, params)?;
        // set stack addr to register
        self.helper.icicle.cpu.write_reg(self.esp, stack_pos);

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
