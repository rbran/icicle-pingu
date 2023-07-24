use icicle_mem::perm;

use crate::{helper, strlen::StrlenTests};

pub fn test_arm<'a, I>(tests_src: &str, tests: &I) -> bool
where
    I: StrlenTests,
{
    let mut vm = icicle_vm::build(&icicle_vm::cpu::Config {
        triple: "armv7-unknown-unknown".parse().unwrap(),
        enable_shadow_stack: false,
        ..icicle_vm::cpu::Config::default()
    })
    .unwrap();
    let r0 = vm.cpu.arch.sleigh.get_reg("r0").unwrap().var;
    let lr = vm.cpu.arch.sleigh.get_reg("lr").unwrap().var;
    let sp = vm.cpu.arch.sleigh.get_reg("sp").unwrap().var;

    let mut success = true;
    for test in 0..2 {
        helper::create_null(&mut vm.cpu.mem).unwrap();
        //write the string somewhere in memory
        let str_addr =
            helper::create_empty_memory(&mut vm.cpu.mem, None, tests.max_len(), perm::READ)
                .unwrap();
        let stack_addr = helper::create_stack(&mut vm.cpu.mem, 0x1000).unwrap();
        let stack_addr_end = stack_addr + 0x1000;

        // write the function
        let file = format!("{}/strlen/strlen-arm-{}.bin", tests_src, test);
        let file_bytes = std::fs::read(&file).unwrap();
        let code_addr = helper::create_empty_memory(
            &mut vm.cpu.mem,
            None,
            file_bytes.len() as u64 + 4,
            perm::EXEC | perm::READ,
        )
        .unwrap();
        let code_return = code_addr + file_bytes.len() as u64;
        //write the code
        vm.cpu
            .mem
            .write_bytes(code_addr, &file_bytes, perm::EXEC | perm::READ)
            .unwrap();
        //write a nop, avoid https://github.com/icicle-emu/icicle-emu/issues/40
        vm.cpu
            .mem
            .write_bytes(
                code_return,
                &[0x00, 0x00, 0xa0, 0xe1],
                perm::EXEC | perm::READ,
            )
            .unwrap();
        for (i, test) in tests.clone().enumerate() {
            // write the test str
            test.write_str(&mut vm.cpu.mem, str_addr);

            // reset the regs
            vm.cpu.write_pc(code_addr);
            vm.cpu.write_reg(sp, stack_addr_end);
            // write param0
            vm.cpu.write_reg(r0, str_addr);
            // write the return addr (after the code)
            vm.cpu.write_reg(lr, code_return);
            if vm.run_until(code_return) != icicle_vm::VmExit::Breakpoint {
                eprintln!("Unable to execute {} with test {}", file, i);
                success &= false;
            } else if vm.cpu.read_reg(r0) != test.result() {
                eprintln!("Invalid result {} with test {}", file, i);
                success &= false;
            }
        }
        vm.reset();
    }
    success
}

pub fn test_thumb<'a, I>(tests_src: &str, tests: &I) -> bool
where
    I: StrlenTests,
{
    let mut vm = icicle_vm::build(&icicle_vm::cpu::Config {
        triple: "thumbv7m-unknown-unknown".parse().unwrap(),
        enable_shadow_stack: false,
        ..icicle_vm::cpu::Config::default()
    })
    .unwrap();
    let r0 = vm.cpu.arch.sleigh.get_reg("r0").unwrap().var;
    let lr = vm.cpu.arch.sleigh.get_reg("lr").unwrap().var;
    let sp = vm.cpu.arch.sleigh.get_reg("sp").unwrap().var;

    let mut success = true;
    for test in 0..2 {
        helper::create_null(&mut vm.cpu.mem).unwrap();
        //write the string somewhere in memory
        let str_addr =
            helper::create_empty_memory(&mut vm.cpu.mem, None, tests.max_len(), perm::READ)
                .unwrap();
        let stack_addr = helper::create_stack(&mut vm.cpu.mem, 0x1000).unwrap();
        let stack_addr_end = stack_addr + 0x1000;

        // write the function
        let code_addr =
            helper::create_empty_memory(&mut vm.cpu.mem, None, 0x1000, perm::EXEC | perm::READ)
                .unwrap();
        let file = format!("{}/strlen/strlen-arm-thumb-{}.bin", tests_src, test);
        let file_bytes = std::fs::read(&file).unwrap();
        let code_return = code_addr + file_bytes.len() as u64;
        //write the code
        vm.cpu
            .mem
            .write_bytes(code_addr, &file_bytes, perm::EXEC | perm::READ)
            .unwrap();
        //write something, avoid https://github.com/icicle-emu/icicle-emu/issues/40
        vm.cpu
            .mem
            .write_bytes(
                code_return,
                &[0x00, 0x00, 0x00, 0x00],
                perm::EXEC | perm::READ,
            )
            .unwrap();
        for (i, test) in tests.clone().enumerate() {
            // write the test str
            test.write_str(&mut vm.cpu.mem, str_addr);

            // reset the regs
            vm.cpu.write_pc(code_addr);
            vm.cpu.write_reg(sp, stack_addr_end);
            // write param0
            vm.cpu.write_reg(r0, str_addr);
            // write the return addr (after the code)
            vm.cpu.write_reg(lr, code_return);
            if vm.run_until(code_return) != icicle_vm::VmExit::Breakpoint {
                eprintln!("Unable to execute {} with test {}", file, i);
                success &= false;
            } else if vm.cpu.read_reg(r0) != test.result() {
                eprintln!("Invalid result {} with test {}", file, i);
                success &= false;
            }
        }
        vm.reset();
    }
    success
}
