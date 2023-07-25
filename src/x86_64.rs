use icicle_mem::perm;

use crate::vm::IcicleHelper;

use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use icicle_vm;
use pcode::VarNode;

use crate::vm::{Param, Vm};

pub struct X86_64 {
    pub helper: IcicleHelper,
    rax: VarNode,
    rsp: VarNode,
    rdi: VarNode,
}

impl X86_64 {
    pub fn new(musl: &Path) -> Result<Self> {
        let mut vm = icicle_vm::build(&icicle_vm::cpu::Config {
            triple: "x86_64-linux-musl".parse().unwrap(),
            enable_shadow_stack: false,
            ..icicle_vm::cpu::Config::default()
        })?;
        vm.env = icicle_vm::env::build_auto(&mut vm)?;
        vm.env
            .load(&mut vm.cpu, musl.as_os_str().as_bytes())
            .map_err(|e| anyhow!(e))?;

        let rax = vm.cpu.arch.sleigh.get_reg("RAX").unwrap().var;
        let rsp = vm.cpu.arch.sleigh.get_reg("RSP").unwrap().var;
        let rdi = vm.cpu.arch.sleigh.get_reg("RDI").unwrap().var;
        Ok(Self {
            helper: IcicleHelper::new(
                vm,
                0x1000_0000,
                0x1000_0000,
                0x2000_0000,
                0x1000_0000,
            ),
            rax,
            rsp,
            rdi,
        })
    }

    fn stack_used(params: &[Param]) -> u64 {
        params
            .iter()
            .enumerate()
            .map(|(idx, param)| match (idx, param) {
                (0, Param::Usize(_)) => 0,
                (0, Param::HeapData(_)) => 0,
                //(0, Param::StackData(data)) => data.len() as u64,
                (0, Param::HeapFn(_, _)) => 0,
                //(0, Param::StackFn(len, _)) => *len,
                _ => todo!(),
            })
            .sum::<u64>()
            // + 8 for the return address added to the stack
            + 8
    }

    fn set_stack(
        &mut self,
        stack_pos: &mut u64,
        return_addr: u64,
        params: &mut [Param],
    ) -> Result<()> {
        for (idx, param) in params.iter_mut().enumerate() {
            // TODO: https://gitlab.com/x86-psABIs/x86-64-ABI/-/jobs/artifacts/master/raw/x86-64-ABI/abi.pdf?job=build
            match (idx, param) {
                (0, Param::Usize(value)) => {
                    self.helper.icicle.cpu.write_reg(self.rdi, *value)
                }
                (0, Param::HeapData(data)) => {
                    let addr = self.helper.malloc(data.len() as u64)?;
                    // write the heap
                    self.helper.icicle.cpu.mem.write_bytes(
                        addr,
                        data,
                        perm::NONE,
                    )?;
                    // put the addr to the reg
                    self.helper.icicle.cpu.write_reg(self.rdi, addr)
                }
                (0, Param::HeapFn(len, write_data)) => {
                    *stack_pos -= 8;
                    let addr = self.helper.malloc(*len)?;
                    write_data(&mut self.helper.icicle.cpu.mem, addr)?;
                    // put the addr to the reg
                    self.helper.icicle.cpu.write_reg(self.rdi, addr)
                }
                _ => todo!(),
            }
        }

        // add the return addr to the stack
        *stack_pos -= 8;
        self.helper
            .icicle
            .cpu
            .mem
            .write_u64(*stack_pos, return_addr, perm::NONE)
            .unwrap();

        Ok(())
    }

    fn get_results(&mut self, results: &mut [Param]) -> Result<()> {
        match results {
            [] => {}
            [Param::Usize(value)] => {
                *value = self.helper.icicle.cpu.read_reg(self.rax)
            }
            _ => todo!(),
        }
        Ok(())
    }
}

impl Vm for X86_64 {
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
        self.set_stack(&mut stack_pos, return_addr, params)?;
        // set stack addr to register
        self.helper.icicle.cpu.write_reg(self.rsp, stack_pos);

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
