//! Independent access oracle: Linux page permissions / Cortex-M4 MPU.
//! Production TD is unmodified. Denied accesses terminate the witness, including
//! stores of an unchanged byte and scans whose result never escapes the cursor.
use crate::{heap, report};

#[derive(Clone, Copy, Default)]
pub struct Region {
    pub base: usize,
    pub bytes: usize,
}
#[derive(Clone, Copy)]
pub enum Permission {
    ReadOnly,
    None,
}

pub struct Guard {
    regions: [Region; 8],
    count: usize,
}
impl Guard {
    pub fn new(regions: impl Iterator<Item = Region>, permission: Permission) -> Self {
        let mut guard = Self {
            regions: [Region::default(); 8],
            count: 0,
        };
        for region in regions {
            assert!(
                guard.count < guard.regions.len(),
                "MPU region catalog exceeded"
            );
            assert!(region.bytes >= 4096 && region.bytes.is_power_of_two());
            assert_eq!(region.base % region.bytes, 0);
            guard.regions[guard.count] = region;
            guard.count += 1;
        }
        platform::protect(&guard.regions[..guard.count], permission);
        guard
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        platform::restore(&self.regions[..self.count]);
    }
}

pub fn self_test() {
    let value = heap::isolated_source(|| alloc::vec![0x5au8]);
    let region = heap::byte_regions(true).next().unwrap();
    assert!((region.base..region.base + region.bytes).contains(&(value.as_ptr() as usize)));
    for permission in [Permission::ReadOnly, Permission::None] {
        platform::assert_fault(region, value.as_ptr() as *mut u8, permission);
    }
    assert_eq!(value[0], 0x5a);
    drop(value);
    heap::assert_empty();
    report(format_args!(
        "access oracle: denied read and same-value write controls passed"
    ));
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    unsafe extern "C" {
        fn mprotect(base: *mut u8, bytes: usize, prot: i32) -> i32;
        fn getpagesize() -> i32;
        fn fork() -> i32;
        fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    }
    pub fn protect(regions: &[Region], permission: Permission) {
        assert_eq!(
            unsafe { getpagesize() },
            4096,
            "unsupported native page size"
        );
        let prot = match permission {
            Permission::ReadOnly => 1,
            Permission::None => 0,
        };
        for r in regions {
            assert_eq!(unsafe { mprotect(r.base as *mut u8, r.bytes, prot) }, 0);
        }
    }
    pub fn restore(regions: &[Region]) {
        for r in regions {
            assert_eq!(unsafe { mprotect(r.base as *mut u8, r.bytes, 3) }, 0);
        }
    }
    pub fn assert_fault(region: Region, pointer: *mut u8, permission: Permission) {
        let pid = unsafe { fork() };
        assert!(pid >= 0);
        if pid == 0 {
            let _guard = Guard::new(core::iter::once(region), permission);
            unsafe {
                match permission {
                    Permission::None => {
                        core::hint::black_box(pointer.read_volatile());
                    }
                    Permission::ReadOnly => {
                        assert_eq!(pointer.read_volatile(), 0x5a);
                        pointer.write_volatile(0x5a);
                    }
                }
            }
            crate::exit(2); // a silently ineffective guard must fail the parent
        }
        let mut status = 0;
        assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
        assert_eq!(status & 0x7f, 11, "access control did not raise SIGSEGV");
    }
}

#[cfg(target_os = "none")]
mod platform {
    use super::*;
    use core::sync::atomic::Ordering;
    const MPU_TYPE: *const u32 = 0xe000_ed90usize as _;
    const MPU_CTRL: *mut u32 = 0xe000_ed94usize as _;
    const MPU_RNR: *mut u32 = 0xe000_ed98usize as _;
    const MPU_RBAR: *mut u32 = 0xe000_ed9cusize as _;
    const MPU_RASR: *mut u32 = 0xe000_eda0usize as _;
    const CFSR: *mut u32 = 0xe000_ed28usize as _;
    const HFSR: *mut u32 = 0xe000_ed2cusize as _;
    const MMFAR: *const u32 = 0xe000_ed34usize as _;

    pub fn protect(regions: &[Region], permission: Permission) {
        unsafe {
            assert!(MPU_TYPE.read_volatile() >> 8 & 0xff >= 8);
            MPU_CTRL.write_volatile(0);
            core::arch::asm!("dsb", "isb");
            for i in 0..8 {
                MPU_RNR.write_volatile(i);
                MPU_RASR.write_volatile(0);
            }
            for (i, region) in regions.iter().enumerate() {
                MPU_RNR.write_volatile(i as u32);
                MPU_RBAR.write_volatile(region.base as u32);
                // XN, normal noncacheable memory (TEX=1), AP=read-only or
                // no access, power-of-two SIZE, ENABLE. No subregions overlap.
                let ap = match permission {
                    Permission::ReadOnly => 6,
                    Permission::None => 0,
                };
                let size = region.bytes.trailing_zeros() - 1;
                MPU_RASR.write_volatile((1 << 28) | (ap << 24) | (1 << 19) | (size << 1) | 1);
            }
            MPU_CTRL.write_volatile(5); // ENABLE + privileged default map
            core::arch::asm!("dsb", "isb");
        }
    }
    pub fn restore(_: &[Region]) {
        unsafe {
            MPU_CTRL.write_volatile(0);
            core::arch::asm!("dsb", "isb");
        }
    }
    pub fn assert_fault(region: Region, pointer: *mut u8, permission: Permission) {
        let controls = heap::access_controls();
        controls.expected.store(pointer as usize, Ordering::SeqCst);
        controls.observed.store(0, Ordering::SeqCst);
        let guard = Guard::new(core::iter::once(region), permission);
        unsafe {
            match permission {
                Permission::None => {
                    core::hint::black_box(pointer.read_volatile());
                }
                Permission::ReadOnly => {
                    assert_eq!(pointer.read_volatile(), 0x5a);
                    pointer.write_volatile(0x5a);
                }
            }
        }
        drop(guard);
        controls.expected.store(0, Ordering::SeqCst);
        assert_eq!(controls.observed.load(Ordering::SeqCst), pointer as usize);
    }
    // Recovery is confined to the two positive controls. Any production access
    // fault fails immediately. The controls verify DACCVIOL + valid MMFAR,
    // then remove protection and retry the faulting load/store on return.
    // PRIMASK remains set: MPU violations escalate to HardFault, which shares
    // this entry. No external interrupt is enabled to observe the heap.
    pub extern "C" fn fault() {
        let flags = unsafe { CFSR.read_volatile() };
        let address = unsafe { MMFAR.read_volatile() } as usize;
        let controls = heap::access_controls();
        if flags == 0x82 && controls.expected.load(Ordering::SeqCst) == address {
            controls.observed.store(address, Ordering::SeqCst);
            restore(&[]);
            unsafe {
                CFSR.write_volatile(flags);
                HFSR.write_volatile(1 << 30); // clear FORCED escalation
            }
        } else {
            report(format_args!(
                "FAIL: forbidden TD access address={address:#x} fault={flags:#x}"
            ));
            crate::exit(1);
        }
    }
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn AccessFault() {
    platform::fault();
}
