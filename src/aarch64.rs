use icicle_mem::perm;

use crate::vm::IcicleHelper;

use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use icicle_vm;
use pcode::VarNode;

use crate::vm::{Param, Vm};

pub struct Aarch64 {
    pub helper: IcicleHelper,
    x0: VarNode,
    w0: VarNode,
    x30: VarNode,
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

        let x0 = vm.cpu.arch.sleigh.get_reg("x0").unwrap().var;
        let w0 = vm.cpu.arch.sleigh.get_reg("w0").unwrap().var;
        let x30 = vm.cpu.arch.sleigh.get_reg("x30").unwrap().var;
        let sp = vm.cpu.arch.sleigh.get_reg("sp").unwrap().var;
        Ok(Self {
            helper: IcicleHelper::new(
                vm,
                0x1000_0000,
                0x1000_0000,
                0x2000_0000,
                0x1000_0000,
            ),
            x0,
            w0,
            x30,
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
        return_addr: u64,
        params: &mut [Param],
    ) -> Result<()> {
        for (idx, param) in params.iter_mut().enumerate() {
            // TODO: https://gitlab.com/x86-psABIs/x86-64-ABI/-/jobs/artifacts/master/raw/x86-64-ABI/abi.pdf?job=build
            match (idx, param) {
                (0, Param::Usize(value)) => {
                    self.helper.icicle.cpu.write_reg(self.x0, *value)
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
                    self.helper.icicle.cpu.write_reg(self.x0, addr)
                }
                (0, Param::HeapFn(len, write_data)) => {
                    let addr = self.helper.malloc(*len)?;
                    write_data(&mut self.helper.icicle.cpu.mem, addr)?;
                    // put the addr to the reg
                    self.helper.icicle.cpu.write_reg(self.x0, addr)
                }
                _ => todo!(),
            }
        }

        // write the return addr to w0
        self.helper.icicle.cpu.write_reg(self.x30, return_addr);
        Ok(())
    }

    fn get_results(&mut self, results: &mut [Param]) -> Result<()> {
        match results {
            [] => {}
            [Param::Usize(value)] => {
                *value = self.helper.icicle.cpu.read_reg(self.w0)
            }
            _ => todo!(),
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
        results: &mut [Param],
    ) -> Result<()> {
        //clean the heap
        self.helper.free_all();

        let stack_len = Self::stack_used(params);
        self.helper.set_stack_len(stack_len)?;

        let mut stack_pos = self.helper.stack_addr + self.helper.stack_size;
        self.set_stack(&mut stack_pos, return_addr, params)?;
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
//use icicle_mem::perm;
//
//use crate::{helper, strlen::StrlenTests};
//
//pub fn test<'a, I>(tests_src: &str, tests: &I) -> bool
//where
//    I: StrlenTests,
//{
//    let mut vm = icicle_vm::build(&icicle_vm::cpu::Config {
//        triple: "aarch64-unknown-unknown".parse().unwrap(),
//        enable_shadow_stack: false,
//        ..icicle_vm::cpu::Config::default()
//    })
//    .unwrap();
//    let x0 = vm.cpu.arch.sleigh.get_reg("x0").unwrap().var;
//    let w0 = vm.cpu.arch.sleigh.get_reg("w0").unwrap().var;
//    let x30 = vm.cpu.arch.sleigh.get_reg("x30").unwrap().var;
//    let sp = vm.cpu.arch.sleigh.get_reg("sp").unwrap().var;
//
//    let mut success = true;
//    for test in 0..2 {
//        helper::create_null(&mut vm.cpu.mem).unwrap();
//        //write the string somewhere in memory
//        let str_addr =
//            helper::create_empty_memory(&mut vm.cpu.mem, None, tests.max_len(), perm::READ)
//                .unwrap();
//        let stack_addr = helper::create_stack(&mut vm.cpu.mem, 0x1000).unwrap();
//        let stack_addr_end = stack_addr + 0x1000;
//
//        // write the function
//        let file = format!("{}/strlen/strlen-aarch64-{}.bin", tests_src, test);
//        let file_bytes = std::fs::read(&file).unwrap();
//        let code_addr = helper::create_empty_memory(
//            &mut vm.cpu.mem,
//            None,
//            file_bytes.len() as u64 + 4, //+4 for the nop
//            perm::EXEC | perm::READ,
//        )
//        .unwrap();
//        let code_return = code_addr + file_bytes.len() as u64;
//        //write the code
//        vm.cpu
//            .mem
//            .write_bytes(code_addr, &file_bytes, perm::EXEC | perm::READ)
//            .unwrap();
//        //write a nop, avoid https://github.com/icicle-emu/icicle-emu/issues/40
//        vm.cpu
//            .mem
//            .write_bytes(
//                code_return,
//                &[0x1f, 0x20, 0x03, 0xd5],
//                perm::EXEC | perm::READ,
//            )
//            .unwrap();
//        for (i, test) in tests.clone().enumerate() {
//            // write the test str
//            test.write_str(&mut vm.cpu.mem, str_addr);
//
//            // reset the regs
//            vm.cpu.write_pc(code_addr);
//            vm.cpu.write_reg(sp, stack_addr_end);
//            // write param0
//            vm.cpu.write_reg(x0, str_addr);
//            // write the return addr (after the code)
//            vm.cpu.write_reg(x30, code_return);
//            if vm.run_until(code_return) != icicle_vm::VmExit::Breakpoint {
//                eprintln!("Unable to execute {} with test {}", file, i);
//                success &= false;
//            } else if vm.cpu.read_reg(w0) != test.result() {
//                eprintln!("Invalid result {} with test {}", file, i);
//                success &= false;
//            }
//        }
//        vm.reset();
//    }
//    success
//}
