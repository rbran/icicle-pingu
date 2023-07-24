mod aarch64;
mod arm;
mod helper;
mod strlen;
mod x86;
mod x86_64;

fn main() {
    let src = "tests";
    let tests = strlen::all_tests();
    if !x86::test_i386(&src, &tests) {
        eprintln!("Fail i386");
    }
    if !x86::test_i586(&src, &tests) {
        eprintln!("Fail i586");
    }
    if !x86_64::test(&src, &tests) {
        eprintln!("Fail x86_64");
    }
    if !arm::test_arm(&src, &tests) {
        eprintln!("Fail arm");
    }
    if !arm::test_thumb(&src, &tests) {
        eprintln!("Fail thumb");
    }
    if !aarch64::test(&src, &tests) {
        eprintln!("Fail aarch64");
    }
}
