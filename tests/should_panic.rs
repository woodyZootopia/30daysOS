#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]

use core::panic::PanicInfo;

use haribote::serial_println;
use haribote::{exit_qemu, QemuExitCode};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    correct_panic();
    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]\n");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

fn correct_panic() {
    panic!("PANIC!");
}
