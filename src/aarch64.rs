use icicle_mem::perm;

use crate::{helper, strlen::StrlenTests};

pub fn test<'a, I>(tests_src: &str, tests: &I) -> bool
where
    I: StrlenTests,
{
    let mut vm = icicle_vm::build(&icicle_vm::cpu::Config {
        triple: "aarch64-unknown-unknown".parse().unwrap(),
        enable_shadow_stack: false,
        ..icicle_vm::cpu::Config::default()
    })
    .unwrap();
    let x0 = vm.cpu.arch.sleigh.get_reg("x0").unwrap().var;
    let w0 = vm.cpu.arch.sleigh.get_reg("w0").unwrap().var;
    let x30 = vm.cpu.arch.sleigh.get_reg("x30").unwrap().var;
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
        let file = format!("{}/strlen/strlen-aarch64-{}.bin", tests_src, test);
        let file_bytes = std::fs::read(&file).unwrap();
        let code_addr = helper::create_empty_memory(
            &mut vm.cpu.mem,
            None,
            file_bytes.len() as u64 + 4, //+4 for the nop
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
                &[0x1f, 0x20, 0x03, 0xd5],
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
            vm.cpu.write_reg(x0, str_addr);
            // write the return addr (after the code)
            vm.cpu.write_reg(x30, code_return);
            if vm.run_until(code_return) != icicle_vm::VmExit::Breakpoint {
                eprintln!("Unable to execute {} with test {}", file, i);
                success &= false;
            } else if vm.cpu.read_reg(w0) != test.result() {
                eprintln!("Invalid result {} with test {}", file, i);
                success &= false;
            }
        }
        vm.reset();
    }
    success
}
