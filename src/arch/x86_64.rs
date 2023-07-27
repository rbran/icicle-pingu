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
    r: [VarNode; 6],
    xmm_qa: [VarNode; 6],
    xmm_da: [VarNode; 6],
    rax: VarNode,
    rsp: VarNode,
}

impl X86_64 {
    const fn regs(idx: usize) -> &'static str {
        match idx {
            0 => "RDI",
            1 => "RSI",
            2 => "RDX",
            3 => "RCX",
            4 => "R8",
            5 => "R9",
            _ => unreachable!(),
        }
    }

    fn xmm_qa(idx: usize) -> String {
        format!("XMM{}_Qa", idx)
    }

    fn xmm_da(idx: usize) -> String {
        format!("XMM{}_Qa", idx)
    }

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

        let r = (0..6)
            .map(|i| vm.cpu.arch.sleigh.get_reg(Self::regs(i)).unwrap().var)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let xmm_qa = (0..6)
            .map(|i| vm.cpu.arch.sleigh.get_reg(&Self::xmm_qa(i)).unwrap().var)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let xmm_da = (0..6)
            .map(|i| vm.cpu.arch.sleigh.get_reg(&Self::xmm_da(i)).unwrap().var)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
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
            r,
            xmm_qa,
            xmm_da,
        })
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
        //TODO min len for the stack
        let stack_len = Self::stack_used(params).max(0x1000);
        self.helper.set_stack_len(stack_len)?;

        let mut stack_pos = self.helper.stack_addr + self.helper.stack_size;
        // TODO: https://gitlab.com/x86-psABIs/x86-64-ABI/-/jobs/artifacts/master/raw/x86-64-ABI/abi.pdf?job=build
        for (i, param) in params.iter_mut().enumerate() {
            match param {
                Param::Usize(value) => {
                    self.helper.icicle.cpu.write_reg(self.r[i], *value)
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
                    self.helper.icicle.cpu.write_reg(self.r[i], addr)
                }
                Param::HeapFn(write_data) => {
                    let addr = write_data(&mut self.helper)?;
                    // put the addr to the reg
                    self.helper.icicle.cpu.write_reg(self.r[i], addr)
                }
                Param::F32(value) => self
                    .helper
                    .icicle
                    .cpu
                    .write_reg(self.xmm_da[i], value.to_bits() as u64),
                Param::F64(value) => self
                    .helper
                    .icicle
                    .cpu
                    .write_reg(self.xmm_qa[i], value.to_bits()),
                Param::I64(value) => {
                    self.helper.icicle.cpu.write_reg(self.r[i], *value as u64)
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
            Return::F32(value) => {
                *value = f32::from_bits(
                    self.helper.icicle.cpu.read_reg(self.xmm_da[0]) as u32,
                )
            }
            Return::F64(value) => {
                *value = f64::from_bits(
                    self.helper.icicle.cpu.read_reg(self.xmm_qa[0]),
                )
            }
            Return::I64(value) => {
                *value = self.helper.icicle.cpu.read_reg(self.rax) as i64
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
