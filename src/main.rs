#![no_std]
#![no_main]
#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(lib::test_runner)]

extern crate alloc;
extern crate rlibc;

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use haribote as lib;
use lib::println;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    let _phys_mem_offset = haribote::init(boot_info);

    #[cfg(test)]
    {
        use lib::serial_println;
        serial_println!("{}", "#".repeat(100));
        serial_println!("memory_offset:{:?}", _phys_mem_offset);
        serial_println!("Displaying memory regions...");
        boot_info
            .memory_map
            .iter()
            .for_each(|x| serial_println!("{:?}", x));
        serial_println!("{}", "#".repeat(100));

        lib::exit_qemu(lib::QemuExitCode::Success);
    }

    haribote::vga_graphic::graphic_mode();

    println!("It did not crash!");

    haribote::kernel_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    haribote::test_panic_handler(info)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    lib::asm::cli();
    use vga::writers::{Text80x25, TextWriter};
    let textmode = Text80x25::new();
    textmode.set_mode();
    for _ in 0..24 {
        println!();
    }
    println!("{}", info);
    haribote::hlt_loop();
}
