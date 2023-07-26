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
    w: [VarNode; 31],
    d: [VarNode; 8],
    s: [VarNode; 8],
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

        let w = (0..31)
            .map(|reg| {
                vm.cpu
                    .arch
                    .sleigh
                    .get_reg(&format!("w{}", reg))
                    .unwrap()
                    .var
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let d = (0..=7)
            .map(|reg| {
                vm.cpu
                    .arch
                    .sleigh
                    .get_reg(&format!("d{}", reg))
                    .unwrap()
                    .var
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let s = (0..=7)
            .map(|reg| {
                vm.cpu
                    .arch
                    .sleigh
                    .get_reg(&format!("s{}", reg))
                    .unwrap()
                    .var
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let sp = vm.cpu.arch.sleigh.get_reg("sp").unwrap().var;
        let helper = IcicleHelper::new(
            vm,
            0x1000_0000,
            0x1000_0000,
            0x2000_0000,
            0x1000_0000,
        );
        Ok(Self {
            helper,
            w,
            d,
            s,
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
        return_addr: u64,
        params: &mut [Param],
    ) -> Result<u64> {
        //TODO min len for the stack
        let stack_len = Self::stack_used(params).max(0x1000);
        self.helper.set_stack_len(stack_len)?;

        let stack_pos = self.helper.stack_addr + self.helper.stack_size;

        if params.len() > 7 {
            todo!()
        }
        for (i, param) in params.iter_mut().enumerate() {
            match param {
                Param::Usize(value) => {
                    self.helper.icicle.cpu.write_reg(self.w[i], *value)
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
                    self.helper.icicle.cpu.write_reg(self.w[i], addr)
                }
                Param::HeapFn(write_data) => {
                    let addr = write_data(&mut self.helper)?;
                    // put the addr to the reg
                    self.helper.icicle.cpu.write_reg(self.w[i], addr)
                }
                Param::F32(value) => self
                    .helper
                    .icicle
                    .cpu
                    .write_reg(self.s[i], value.to_bits() as u64),
                Param::F64(value) => {
                    self.helper.icicle.cpu.write_reg(self.d[i], value.to_bits())
                }
            }
        }

        // write the return addr to x30/LR
        self.helper.icicle.cpu.write_reg(self.w[30], return_addr);
        Ok(stack_pos)
    }

    fn get_results(&mut self, results: &mut [Return]) -> Result<()> {
        if results.len() > 7 {
            todo!()
        }
        for (i, result) in results.iter_mut().enumerate() {
            match result {
                Return::Usize(value) => {
                    *value = self.helper.icicle.cpu.read_reg(self.w[i])
                }
                Return::CString(data) => {
                    let addr = self.helper.icicle.cpu.read_reg(self.w[i]);
                    self.helper.icicle.cpu.mem.read_cstr(addr, data)?;
                }
                Return::F32(value) => {
                    *value = f32::from_bits(
                        self.helper.icicle.cpu.read_reg(self.s[i]) as u32,
                    )
                }
                Return::F64(value) => {
                    *value = f64::from_bits(
                        self.helper.icicle.cpu.read_reg(self.d[i]),
                    )
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

        let stack_pos = self.set_call(return_addr, params)?;
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
