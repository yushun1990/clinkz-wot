//! Actual no_std + alloc execution of the production public TD boundary.
#![no_std]
#![no_main]

extern crate alloc;

mod access;
mod heap;
mod scenarios;

use core::fmt::{self, Write};

#[global_allocator]
static HEAP: heap::Allocator = heap::Allocator;

struct Console;
impl Write for Console {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        #[cfg(target_os = "linux")]
        unsafe {
            unsafe extern "C" {
                fn write(fd: i32, bytes: *const u8, count: usize) -> isize;
            }
            let mut bytes = text.as_bytes();
            while !bytes.is_empty() {
                let n = write(1, bytes.as_ptr(), bytes.len());
                if n <= 0 {
                    return Err(fmt::Error);
                }
                bytes = &bytes[n as usize..];
            }
        }
        #[cfg(target_os = "none")]
        for bytes in text.as_bytes().chunks(255) {
            let mut message = [0u8; 256];
            message[..bytes.len()].copy_from_slice(bytes);
            unsafe { semihost(4, message.as_ptr() as usize) };
        }
        Ok(())
    }
}
pub(crate) fn report(args: fmt::Arguments<'_>) {
    Console.write_fmt(args).unwrap();
    Console.write_str("\n").unwrap();
}

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    scenarios::run();
    0
}

// Linux's prebuilt liballoc contains unwind references even with panic=abort.
// No unwinding is supported: either unexpected landing point fails the run.
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() -> ! {
    exit(1)
}
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
extern "C" fn _Unwind_Resume() -> ! {
    exit(1)
}

#[cfg(target_os = "none")]
core::arch::global_asm!(
    ".section .vector_table,\"a\"",
    ".word _stack_start",
    ".word Reset",
    ".word Fault",       // NMI
    ".word AccessFault", // HardFault (MPU faults with PRIMASK set)
    ".word AccessFault", // MemManage
    ".rept 11",
    ".word Fault",
    ".endr",
);

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
unsafe extern "C" fn Reset() -> ! {
    unsafe extern "C" {
        static mut _sbss: u8;
        static mut _ebss: u8;
        static mut _sdata: u8;
        static mut _edata: u8;
        static _sidata: u8;
    }
    unsafe {
        // Single caller, no interrupts or allocator reentry. Enable the M4 FPU
        // before entering the hard-float Rust dependency graph.
        core::arch::asm!("cpsid i");
        (0xe000_ed88 as *mut u32).write_volatile(0x00f0_0000);
        core::arch::asm!("dsb", "isb");
        let start = &raw mut _sbss;
        let len = (&raw mut _ebss as usize) - (start as usize);
        core::ptr::write_bytes(start, 0, len);
        let start = &raw mut _sdata;
        let len = (&raw mut _edata as usize) - (start as usize);
        core::ptr::copy_nonoverlapping(&raw const _sidata, start, len);
    }
    scenarios::run();
    exit(0)
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
extern "C" fn Fault() -> ! {
    let (cfsr, hfsr, mmfar) = unsafe {
        (
            (0xe000_ed28 as *const u32).read_volatile(),
            (0xe000_ed2c as *const u32).read_volatile(),
            (0xe000_ed34 as *const u32).read_volatile(),
        )
    };
    report(format_args!(
        "FAIL: ARM exception cfsr={cfsr:#x} hfsr={hfsr:#x} mmfar={mmfar:#x}"
    ));
    exit(1)
}

#[cfg(target_os = "none")]
unsafe fn semihost(operation: usize, argument: usize) {
    unsafe {
        core::arch::asm!("bkpt 0xab", inout("r0") operation => _, in("r1") argument,
            options(nostack));
    }
}

fn exit(code: i32) -> ! {
    #[cfg(target_os = "linux")]
    unsafe {
        unsafe extern "C" {
            fn _exit(code: i32) -> !;
        }
        _exit(code)
    }
    #[cfg(target_os = "none")]
    unsafe {
        let block = [0x20026usize, code as usize];
        semihost(0x20, block.as_ptr() as usize);
        loop {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    report(format_args!("FAIL: {info}"));
    exit(1)
}
