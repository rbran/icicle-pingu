use icicle_mem::perm;

use crate::{helper, strlen::StrlenTests};

pub fn test_i386<'a, I>(tests_src: &str, tests: &I) -> bool
where
    I: StrlenTests,
{
    test(tests_src, tests, "i386", 4)
}

pub fn test_i586<'a, I>(tests_src: &str, tests: &I) -> bool
where
    I: StrlenTests,
{
    test(tests_src, tests, "i586", 2)
}

fn test<'a, I>(tests_src: &str, tests: &I, arch: &'static str, code_num: usize) -> bool
where
    I: StrlenTests,
{
    let mut vm = icicle_vm::build(&icicle_vm::cpu::Config {
        triple: format!("{}-unknown-unknown", arch).parse().unwrap(),
        enable_shadow_stack: false,
        ..icicle_vm::cpu::Config::default()
    })
    .unwrap();
    let eax = vm.cpu.arch.sleigh.get_reg("EAX").unwrap().var;
    let esp = vm.cpu.arch.sleigh.get_reg("ESP").unwrap().var;

    let mut success = true;
    for code in 0..code_num {
        helper::create_null(&mut vm.cpu.mem).unwrap();
        //write the string somewhere in memory
        let str_addr =
            helper::create_empty_memory(&mut vm.cpu.mem, None, tests.max_len(), perm::READ)
                .unwrap();
        let stack_addr = helper::create_stack(&mut vm.cpu.mem, 0x1000).unwrap();
        let stack_addr_end = stack_addr + 0x1000;

        // write param0
        vm.cpu
            .mem
            .write_u32(stack_addr_end - 4, str_addr as u32, perm::NONE)
            .unwrap();
        let file = format!("{}/strlen/strlen-{}-{}.bin", tests_src, arch, code);
        let file_bytes = std::fs::read(&file).unwrap();
        // write the function
        let code_addr = helper::create_empty_memory(
            &mut vm.cpu.mem,
            None,
            file_bytes.len().try_into().unwrap(),
            perm::EXEC | perm::READ,
        )
        .unwrap();
        let code_return = code_addr + file_bytes.len() as u64;
        // write the return addr (after the code)
        vm.cpu
            .mem
            .write_u32(
                stack_addr_end - 8,
                code_return.try_into().unwrap(),
                perm::NONE,
            )
            .unwrap();
        //write the code
        vm.cpu
            .mem
            .write_bytes(code_addr, &file_bytes, perm::EXEC | perm::READ)
            .unwrap();
        for (i, test) in tests.clone().enumerate() {
            // write the test str
            test.write_str(&mut vm.cpu.mem, str_addr);

            // reset the regs
            vm.cpu.write_pc(code_addr);
            vm.cpu.write_reg(esp, stack_addr_end - 8);
            if vm.run_until(code_return) != icicle_vm::VmExit::Breakpoint {
                eprintln!("Unable to execute {} with test {}", file, i);
                success &= false;
            } else if vm.cpu.read_reg(eax) != test.result() {
                eprintln!("Invalid result {} with test {}", file, i);
                success &= false;
            }
        }
        vm.reset();
    }
    success
}
