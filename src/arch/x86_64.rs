use icicle_mem::perm;

use crate::vm::{IcicleHelper, Return};

use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use icicle_vm;
use pcode::VarNode;

use crate::vm::{Param, Vm};

pub struct X86_64 {
    pub helper: IcicleHelper,
    rdi: VarNode,
    rsi: VarNode,
    rdx: VarNode,
    rcx: VarNode,
    r8: VarNode,
    r9: VarNode,
    rax: VarNode,
    rsp: VarNode,
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

        let rdi = vm.cpu.arch.sleigh.get_reg("RDI").unwrap().var;
        let rsi = vm.cpu.arch.sleigh.get_reg("RSI").unwrap().var;
        let rdx = vm.cpu.arch.sleigh.get_reg("RDX").unwrap().var;
        let rcx = vm.cpu.arch.sleigh.get_reg("RCX").unwrap().var;
        let r8 = vm.cpu.arch.sleigh.get_reg("R8").unwrap().var;
        let r9 = vm.cpu.arch.sleigh.get_reg("R9").unwrap().var;
        let rax = vm.cpu.arch.sleigh.get_reg("RAX").unwrap().var;
        let rsp = vm.cpu.arch.sleigh.get_reg("RSP").unwrap().var;
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
            rsi,
            rdx,
            rcx,
            r8,
            r9,
        })
    }

    fn reg(&self, idx: usize) -> VarNode {
        match idx {
            0 => self.rdi,
            1 => self.rdi,
            2 => self.rsi,
            3 => self.rdx,
            4 => self.rcx,
            5 => self.r8,
            6 => self.r9,
            _ => todo!(),
        }
    }

    fn stack_used(params: &[Param]) -> u64 {
        if params.len() > 6 {
            todo!()
        }
        // 8 for the return address added to the stack
        8
    }

    fn set_call(
        &mut self,
        return_addr: u64,
        params: &mut [Param],
    ) -> Result<u64> {
        if params.len() > 6 {
            todo!()
        }
        let stack_len = Self::stack_used(params);
        self.helper.set_stack_len(stack_len)?;

        let mut stack_pos = self.helper.stack_addr + self.helper.stack_size;
        // TODO: https://gitlab.com/x86-psABIs/x86-64-ABI/-/jobs/artifacts/master/raw/x86-64-ABI/abi.pdf?job=build
        for (idx, param) in params.iter_mut().enumerate() {
            let reg = self.reg(idx);
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

        // add the return addr to the stack
        stack_pos -= 8;
        self.helper
            .icicle
            .cpu
            .mem
            .write_u64(stack_pos, return_addr, perm::NONE)
            .unwrap();

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
                *value = self.helper.icicle.cpu.read_reg(self.rax)
            }
            Return::CString(data) => {
                let addr = self.helper.icicle.cpu.read_reg(self.rax);
                self.helper.icicle.cpu.mem.read_cstr(addr, data)?;
            }
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
        results: &mut [Return],
    ) -> Result<()> {
        //clean the heap
        self.helper.free_all();

        let stack_addr = self.set_call(return_addr, params)?;
        // set stack addr to register
        self.helper.icicle.cpu.write_reg(self.rsp, stack_addr);

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
