#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use bounded_atomic_number_probe::{
    HARD_MAX, LexemeLimit, Rejection, WORKLOAD_CASES, for_each_case, project,
};
use core::{
    alloc::{GlobalAlloc, Layout},
    arch::{asm, naked_asm},
    cell::UnsafeCell,
    fmt::{self, Write},
    hint::black_box,
    mem::{align_of, size_of},
    panic::PanicInfo,
    ptr::{self, addr_of, addr_of_mut},
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use serde_json::Number;

const WORKLOAD_ID: &str = "WP100-ATOMIC-NUMBER-M4-v1";
const RAW_SCHEMA: &str = "wp100-atomic-number-m4-raw-v1";
const EXPECTED_CASES: usize = WORKLOAD_CASES;
const WARMUPS: usize = 100;
const SAMPLES: usize = 1_000;
const CYCLE_LIMIT: u32 = 168_000;
const STACK_LIMIT: usize = 4_096;

const PROJECT_GUARD_BYTES: usize = 256;
const PROJECT_PAINT_BYTES: usize = 8 * 1024;
const PROJECT_STACK_BYTES: usize = PROJECT_GUARD_BYTES + PROJECT_PAINT_BYTES;
const IRQ_GUARD_BYTES: usize = 256;
const IRQ_PAINT_BYTES: usize = 2 * 1024;
const IRQ_STACK_BYTES: usize = IRQ_GUARD_BYTES + IRQ_PAINT_BYTES;
const HEAP_BYTES: usize = 48 * 1024;
const OUTPUT_BYTES: usize = 4 * 1024;

const STACK_PATTERN_A: u32 = 0xa55a_6996;
const STACK_PATTERN_B: u32 = !STACK_PATTERN_A;
const GUARD_PATTERN: u32 = 0xd15e_a5ed;

const FLASH_ACR: *mut u32 = 0x4002_3c00 as *mut u32;
const RCC_CR: *mut u32 = 0x4002_3800 as *mut u32;
const RCC_PLLCFGR: *mut u32 = 0x4002_3804 as *mut u32;
const RCC_CFGR: *mut u32 = 0x4002_3808 as *mut u32;
const RCC_AHB1ENR: *mut u32 = 0x4002_3830 as *mut u32;
const RCC_APB1ENR: *mut u32 = 0x4002_3840 as *mut u32;
const GPIOA_IDR: *const u32 = 0x4002_0010 as *const u32;

const TIM2_CR1: *mut u32 = 0x4000_0000 as *mut u32;
const TIM2_DIER: *mut u32 = 0x4000_000c as *mut u32;
const TIM2_SR: *mut u32 = 0x4000_0010 as *mut u32;
const TIM2_EGR: *mut u32 = 0x4000_0014 as *mut u32;
const TIM2_CNT: *mut u32 = 0x4000_0024 as *mut u32;
const TIM2_PSC: *mut u32 = 0x4000_0028 as *mut u32;
const TIM2_ARR: *mut u32 = 0x4000_002c as *mut u32;

const SCB_VTOR: *mut u32 = 0xe000_ed08 as *mut u32;
const SCB_CPUID: *mut u32 = 0xe000_ed00 as *mut u32;
const SCB_CPACR: *mut u32 = 0xe000_ed88 as *mut u32;
const SCB_SHCSR: *mut u32 = 0xe000_ed24 as *mut u32;
const SCB_DEMCR: *mut u32 = 0xe000_edfc as *mut u32;
const DWT_CTRL: *mut u32 = 0xe000_1000 as *mut u32;
const DWT_CYCCNT: *mut u32 = 0xe000_1004 as *mut u32;
const NVIC_ISER0: *mut u32 = 0xe000_e100 as *mut u32;
const NVIC_ICPR0: *mut u32 = 0xe000_e280 as *mut u32;
const NVIC_IPR: *mut u8 = 0xe000_e400 as *mut u8;
const MPU_CTRL: *mut u32 = 0xe000_ed94 as *mut u32;
const MPU_RNR: *mut u32 = 0xe000_ed98 as *mut u32;
const MPU_RBAR: *mut u32 = 0xe000_ed9c as *mut u32;
const MPU_RASR: *mut u32 = 0xe000_eda0 as *mut u32;
const DBGMCU_IDCODE: *mut u32 = 0xe004_2000 as *mut u32;

type Handler = unsafe extern "C" fn();

#[repr(C, align(256))]
struct VectorTable {
    initial_stack: *const u32,
    reset: unsafe extern "C" fn() -> !,
    exceptions: [Handler; 14],
    interrupts: [Handler; 82],
}

unsafe impl Sync for VectorTable {}

const fn exception_vectors() -> [Handler; 14] {
    let mut handlers = [default_handler as Handler; 14];
    handlers[1] = hard_fault as Handler;
    handlers[2] = hard_fault as Handler;
    handlers[3] = hard_fault as Handler;
    handlers[4] = hard_fault as Handler;
    handlers
}

const fn interrupt_vectors() -> [Handler; 82] {
    let mut handlers = [default_handler as Handler; 82];
    handlers[28] = TIM2 as Handler;
    handlers
}

unsafe extern "C" {
    static _stack_top: u32;
    static mut _sdata: u32;
    static _edata: u32;
    static _sidata: u32;
    static mut _sbss: u32;
    static mut _ebss: u32;
}

#[used]
#[unsafe(link_section = ".vector_table")]
static VECTOR_TABLE: VectorTable = VectorTable {
    initial_stack: addr_of!(_stack_top),
    reset: Reset,
    exceptions: exception_vectors(),
    interrupts: interrupt_vectors(),
};

#[repr(align(256))]
struct ProjectStack([u8; PROJECT_STACK_BYTES]);

#[repr(align(256))]
struct IrqStack([u8; IRQ_STACK_BYTES]);

#[repr(align(16))]
struct HeapStorage([u8; HEAP_BYTES]);

static mut PROJECT_STACK: ProjectStack = ProjectStack([0; PROJECT_STACK_BYTES]);
static mut IRQ_STACK: IrqStack = IrqStack([0; IRQ_STACK_BYTES]);
static mut HEAP_STORAGE: HeapStorage = HeapStorage([0; HEAP_BYTES]);
static mut MASKED_SAMPLES: [u32; SAMPLES] = [0; SAMPLES];
static mut IRQ_SAMPLES: [u32; SAMPLES] = [0; SAMPLES];
static mut IRQ_ASSERTION_DELAYS: [u32; SAMPLES] = [0; SAMPLES];
static mut IRQ_LATENCIES: [u32; SAMPLES] = [0; SAMPLES];
static mut OUTPUT_BUFFER: [u8; OUTPUT_BYTES] = [0; OUTPUT_BYTES];

static ALLOCATION_CALLS: AtomicU32 = AtomicU32::new(0);
static CANCELLATION_ASSERTED: AtomicBool = AtomicBool::new(false);
static IRQ_ENTRY_CYCLE: AtomicU32 = AtomicU32::new(0);
static IRQ_EXIT_CYCLE: AtomicU32 = AtomicU32::new(0);

#[repr(C)]
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

#[repr(C)]
struct AllocationHeader {
    start: *mut u8,
    size: usize,
}

struct HeapState {
    head: FreeBlock,
    initialized: bool,
}

struct BoardAllocator {
    locked: AtomicBool,
    state: UnsafeCell<HeapState>,
}

unsafe impl Sync for BoardAllocator {}

impl BoardAllocator {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(HeapState {
                head: FreeBlock {
                    size: 0,
                    next: ptr::null_mut(),
                },
                initialized: false,
            }),
        }
    }

    unsafe fn init(&self, start: *mut u8, size: usize) {
        let _guard = self.lock();
        let state = unsafe { &mut *self.state.get() };
        let aligned = align_up(start as usize, align_of::<FreeBlock>());
        let skipped = aligned - start as usize;
        assert!(size > skipped + size_of::<FreeBlock>());
        let block = aligned as *mut FreeBlock;
        unsafe {
            block.write(FreeBlock {
                size: (size - skipped) & !(align_of::<FreeBlock>() - 1),
                next: ptr::null_mut(),
            });
        }
        state.head.next = block;
        state.initialized = true;
    }

    fn lock(&self) -> AllocatorGuard<'_> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        AllocatorGuard(self)
    }
}

struct AllocatorGuard<'a>(&'a BoardAllocator);

impl Drop for AllocatorGuard<'_> {
    fn drop(&mut self) {
        self.0.locked.store(false, Ordering::Release);
    }
}

#[global_allocator]
static ALLOCATOR: BoardAllocator = BoardAllocator::new();

unsafe impl GlobalAlloc for BoardAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _guard = self.lock();
        let state = unsafe { &mut *self.state.get() };
        if !state.initialized {
            return ptr::null_mut();
        }

        let required = layout.size().max(1);
        let alignment = layout.align().max(align_of::<AllocationHeader>());
        let mut previous: *mut FreeBlock = &mut state.head;
        let mut current = state.head.next;

        while !current.is_null() {
            let block_start = current as usize;
            let block_end = block_start + unsafe { (*current).size };
            let user = align_up(block_start + size_of::<AllocationHeader>(), alignment);
            let requested_end = match user.checked_add(required) {
                Some(end) => end,
                None => return ptr::null_mut(),
            };
            let split = align_up(requested_end, align_of::<FreeBlock>());
            if split <= block_end {
                let remainder = block_end - split;
                let allocation_end;
                if remainder >= size_of::<FreeBlock>() {
                    let next = unsafe { (*current).next };
                    let suffix = split as *mut FreeBlock;
                    unsafe {
                        suffix.write(FreeBlock {
                            size: remainder,
                            next,
                        });
                        (*previous).next = suffix;
                    }
                    allocation_end = split;
                } else {
                    allocation_end = block_end;
                    unsafe {
                        (*previous).next = (*current).next;
                    }
                }

                let header = (user - size_of::<AllocationHeader>()) as *mut AllocationHeader;
                unsafe {
                    header.write(AllocationHeader {
                        start: current.cast(),
                        size: allocation_end - block_start,
                    });
                }
                ALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
                return user as *mut u8;
            }
            previous = current;
            current = unsafe { (*current).next };
        }
        ptr::null_mut()
    }

    unsafe fn dealloc(&self, pointer: *mut u8, _layout: Layout) {
        if pointer.is_null() {
            return;
        }
        let header = unsafe {
            ((pointer as usize - size_of::<AllocationHeader>()) as *const AllocationHeader).read()
        };
        let _guard = self.lock();
        let state = unsafe { &mut *self.state.get() };
        let mut previous: *mut FreeBlock = &mut state.head;
        let mut current = state.head.next;
        while !current.is_null() && (current as usize) < header.start as usize {
            previous = current;
            current = unsafe { (*current).next };
        }

        let restored = header.start.cast::<FreeBlock>();
        unsafe {
            restored.write(FreeBlock {
                size: header.size,
                next: current,
            });
            (*previous).next = restored;
        }

        unsafe {
            if !current.is_null() && restored as usize + (*restored).size == current as usize {
                (*restored).size += (*current).size;
                (*restored).next = (*current).next;
            }
            if !ptr::eq(previous, &state.head)
                && previous as usize + (*previous).size == restored as usize
            {
                (*previous).size += (*restored).size;
                (*previous).next = (*restored).next;
            }
        }
    }
}

const fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Measurement {
    cycles: u32,
    baseline_psp: u32,
    result_tag: u32,
    result_bits_low: u32,
    result_bits_high: u32,
    allocations: u32,
    operation_entry_cycle: u32,
    cancellation_cycle: u32,
    cancellation_latency_cycles: u32,
    cancellation_observed: u32,
    irq_service_cycles: u32,
}

impl Measurement {
    const ZERO: Self = Self {
        cycles: 0,
        baseline_psp: 0,
        result_tag: 0,
        result_bits_low: 0,
        result_bits_high: 0,
        allocations: 0,
        operation_entry_cycle: 0,
        cancellation_cycle: 0,
        cancellation_latency_cycles: u32::MAX,
        cancellation_observed: 0,
        irq_service_cycles: 0,
    };

    fn result_identity(self) -> (u32, u64) {
        (
            self.result_tag,
            u64::from(self.result_bits_low) | (u64::from(self.result_bits_high) << 32),
        )
    }
}

#[repr(C)]
struct InvokeContext {
    project_stack_top: *mut u8,
    irq_stack_top: *mut u8,
    irq_mode: u32,
}

#[unsafe(naked)]
unsafe extern "C" fn invoke_on_dedicated_stacks(
    _number: *const Number,
    _measurement: *mut Measurement,
    _context: *const InvokeContext,
) {
    naked_asm!(
        "cpsid i",
        "push {{r4-r7, lr}}",
        "mov r4, sp",
        "ldr r3, [r2, #0]",
        "ldr r6, [r2, #4]",
        "msr psp, r3",
        "msr msp, r6",
        "movs r3, #2",
        "msr control, r3",
        "isb",
        "bl {entry}",
        "cpsid i",
        "movs r3, #0",
        "msr control, r3",
        "isb",
        "msr msp, r4",
        "pop {{r4-r7, pc}}",
        entry = sym measurement_entry,
    );
}

#[inline(never)]
unsafe extern "C" fn measurement_entry(
    number: *const Number,
    measurement: *mut Measurement,
    context: *const InvokeContext,
) {
    let irq_mode = unsafe { (*context).irq_mode != 0 };
    let limit = LexemeLimit::new(HARD_MAX).expect("hard maximum is valid");
    let allocations_before = ALLOCATION_CALLS.load(Ordering::Relaxed);
    let operation_entry_cycle;

    if irq_mode {
        CANCELLATION_ASSERTED.store(false, Ordering::Relaxed);
        IRQ_ENTRY_CYCLE.store(0, Ordering::Relaxed);
        IRQ_EXIT_CYCLE.store(0, Ordering::Relaxed);
        operation_entry_cycle = cycle_count();
        arm_cancellation_interrupt();
    } else {
        operation_entry_cycle = 0;
    }

    let mut baseline_psp: u32;
    unsafe {
        asm!("mrs {}, psp", out(reg) baseline_psp, options(nomem, nostack, preserves_flags));
    }
    let start = cycle_count();
    if irq_mode {
        unsafe {
            asm!("cpsie i", "isb", options(nomem, nostack, preserves_flags));
        }
    }
    let result = project(unsafe { &*number }, limit);
    let stop = cycle_count();
    unsafe {
        asm!("cpsid i", options(nomem, nostack, preserves_flags));
    }
    black_box(&result);
    if irq_mode {
        disarm_cancellation_interrupt();
    }
    let allocations = ALLOCATION_CALLS
        .load(Ordering::Relaxed)
        .wrapping_sub(allocations_before);
    let (result_tag, result_bits) = match result {
        Ok(value) => (0, value.to_bits()),
        Err(Rejection::Limit) => (1, 0),
        Err(Rejection::InvalidSchema) => (2, 0),
        Err(Rejection::InvalidConfiguration) => (3, 0),
    };
    let cancellation_cycle = IRQ_ENTRY_CYCLE.load(Ordering::Relaxed);
    let irq_exit = IRQ_EXIT_CYCLE.load(Ordering::Relaxed);
    unsafe {
        measurement.write(Measurement {
            cycles: stop.wrapping_sub(start),
            baseline_psp,
            result_tag,
            result_bits_low: result_bits as u32,
            result_bits_high: (result_bits >> 32) as u32,
            allocations,
            operation_entry_cycle,
            cancellation_cycle,
            cancellation_latency_cycles: if cancellation_cycle == 0 {
                u32::MAX
            } else {
                stop.wrapping_sub(cancellation_cycle)
            },
            cancellation_observed: u32::from(CANCELLATION_ASSERTED.load(Ordering::Relaxed)),
            irq_service_cycles: irq_exit.wrapping_sub(cancellation_cycle),
        });
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn TIM2() {
    let entry = cycle_count();
    unsafe {
        TIM2_SR.write_volatile(0);
    }
    IRQ_ENTRY_CYCLE.store(entry, Ordering::Relaxed);
    CANCELLATION_ASSERTED.store(true, Ordering::Relaxed);
    IRQ_EXIT_CYCLE.store(cycle_count(), Ordering::Relaxed);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn Reset() -> ! {
    unsafe {
        let mut source = addr_of!(_sidata);
        let mut destination = addr_of_mut!(_sdata);
        while destination < addr_of!(_edata).cast_mut() {
            destination.write(source.read());
            destination = destination.add(1);
            source = source.add(1);
        }

        let mut bss = addr_of_mut!(_sbss);
        while bss < addr_of_mut!(_ebss) {
            bss.write(0);
            bss = bss.add(1);
        }

        SCB_VTOR.write_volatile(0x0800_0000);
        SCB_CPACR.write_volatile(SCB_CPACR.read_volatile() | (0b1111 << 20));
        asm!("dsb", "isb", options(nomem, nostack, preserves_flags));

        ALLOCATOR.init(addr_of_mut!(HEAP_STORAGE.0).cast::<u8>(), HEAP_BYTES);
    }
    board_main()
}

fn board_main() -> ! {
    unsafe {
        asm!("cpsid i", options(nomem, nostack, preserves_flags));
    }
    configure_clock_168mhz();
    configure_cycle_counter();
    configure_tim2();

    if user_button_pressed() {
        coverage_loop();
    }

    let mut output = Output::new();
    writeln!(
        output,
        "{{\"record\":\"run_start\",\"schema\":\"{RAW_SCHEMA}\",\"workload_id\":\"{WORKLOAD_ID}\",\"expected_cases\":{EXPECTED_CASES},\"warmup_iterations\":{WARMUPS},\"sample_count\":{SAMPLES},\"cycle_limit\":{CYCLE_LIMIT},\"stack_limit_bytes\":{STACK_LIMIT},\"target\":\"thumbv7em-none-eabihf\",\"board\":\"STM32F407G-DISC1\",\"cpu\":\"STM32F407VGT6 Cortex-M4F r0p1\",\"cpuid\":{},\"device_idcode\":{},\"clock_hz\":168000000,\"clock_source\":\"8MHz-HSE-PLL\",\"flash_wait_states\":5,\"flash_prefetch\":true,\"flash_instruction_cache\":true,\"flash_data_cache\":true,\"code_placement\":\"flash\",\"project_stack_placement\":\"sram\",\"main_stack_placement\":\"ccm\",\"allocator\":\"first-fit-coalescing-static-48k-sram-v1\",\"masked_run_interrupts\":\"PRIMASK=1\",\"application_irq_load\":\"one TIM2 update IRQ per projection, 84MHz timer, ARR=32, ISR asserts cancellation and timestamps entry/exit\",\"rcc_cr\":{},\"rcc_pllcfgr\":{},\"rcc_cfgr\":{},\"flash_acr\":{}}}",
        read_register(SCB_CPUID),
        read_register(DBGMCU_IDCODE),
        read_register(RCC_CR),
        read_register(RCC_PLLCFGR),
        read_register(RCC_CFGR),
        read_register(FLASH_ACR),
    )
    .unwrap();
    output.flush();

    let mut case_index = 0usize;
    let mut failed_cases = 0usize;
    for_each_case(|family, text| {
        let failed = run_case(case_index, family, text);
        failed_cases += usize::from(failed);
        case_index += 1;
    });

    let complete = case_index == EXPECTED_CASES;
    let mut output = Output::new();
    writeln!(
        output,
        "{{\"record\":\"run_end\",\"workload_id\":\"{WORKLOAD_ID}\",\"case_count\":{case_index},\"failed_cases\":{failed_cases},\"complete\":{complete}}}"
    )
    .unwrap();
    output.flush();
    semihost_exit(if complete { 0 } else { 1 })
}

fn run_case(index: usize, family: &str, text: &str) -> bool {
    assert!((1..=HARD_MAX).contains(&text.len()));
    let number: Number = serde_json::from_str(text).expect("corpus entry must be a JSON Number");
    assert_eq!(number.as_str(), text);
    let limit = LexemeLimit::new(HARD_MAX).unwrap();
    assert_eq!(limit.check_number(&number), Ok(text));

    paint_stacks(STACK_PATTERN_A, STACK_PATTERN_A);
    reset_flash_caches();
    let masked_cold = measure_once(&number, false);
    let masked_stack_a = inspect_stacks(masked_cold.baseline_psp, STACK_PATTERN_A, false);

    let mut result_mismatches = 0u32;
    let expected_result = masked_cold.result_identity();
    let mut masked_allocations = masked_cold.allocations;
    let mut masked_max = masked_cold.cycles;
    enable_stack_guards();
    for _ in 0..WARMUPS {
        let sample = measure_once_guarded(&number, false);
        masked_allocations = masked_allocations.wrapping_add(sample.allocations);
        result_mismatches += u32::from(sample.result_identity() != expected_result);
    }
    for sample_index in 0..SAMPLES {
        let sample = measure_once_guarded(&number, false);
        masked_allocations = masked_allocations.wrapping_add(sample.allocations);
        result_mismatches += u32::from(sample.result_identity() != expected_result);
        masked_max = masked_max.max(sample.cycles);
        unsafe {
            addr_of_mut!(MASKED_SAMPLES)
                .cast::<u32>()
                .add(sample_index)
                .write(sample.cycles);
        }
    }
    disable_stack_guards();

    paint_stacks(STACK_PATTERN_B, STACK_PATTERN_B);
    let masked_stack_b_measurement = measure_once(&number, false);
    let masked_stack_b = inspect_stacks(
        masked_stack_b_measurement.baseline_psp,
        STACK_PATTERN_B,
        false,
    );
    masked_allocations = masked_allocations.wrapping_add(masked_stack_b_measurement.allocations);
    masked_max = masked_max.max(masked_stack_b_measurement.cycles);
    result_mismatches += u32::from(masked_stack_b_measurement.result_identity() != expected_result);

    paint_stacks(STACK_PATTERN_A, STACK_PATTERN_A);
    reset_flash_caches();
    let irq_cold = measure_once(&number, true);
    let irq_stack_a = inspect_stacks(irq_cold.baseline_psp, STACK_PATTERN_A, true);

    let mut irq_allocations = irq_cold.allocations;
    let mut irq_max = irq_cold.cycles;
    let mut irq_service_max = irq_cold.irq_service_cycles;
    let mut irq_observed = u32::from(irq_cold.cancellation_observed != 0);
    result_mismatches += u32::from(irq_cold.result_identity() != expected_result);
    enable_stack_guards();
    for _ in 0..WARMUPS {
        let sample = measure_once_guarded(&number, true);
        irq_allocations = irq_allocations.wrapping_add(sample.allocations);
        irq_observed += u32::from(sample.cancellation_observed != 0);
        irq_service_max = irq_service_max.max(sample.irq_service_cycles);
        result_mismatches += u32::from(sample.result_identity() != expected_result);
    }
    for sample_index in 0..SAMPLES {
        let sample = measure_once_guarded(&number, true);
        irq_allocations = irq_allocations.wrapping_add(sample.allocations);
        irq_observed += u32::from(sample.cancellation_observed != 0);
        irq_max = irq_max.max(sample.cycles);
        irq_service_max = irq_service_max.max(sample.irq_service_cycles);
        result_mismatches += u32::from(sample.result_identity() != expected_result);
        let assertion_delay = if sample.cancellation_observed != 0 {
            sample
                .cancellation_cycle
                .wrapping_sub(sample.operation_entry_cycle)
        } else {
            u32::MAX
        };
        unsafe {
            addr_of_mut!(IRQ_SAMPLES)
                .cast::<u32>()
                .add(sample_index)
                .write(sample.cycles);
            addr_of_mut!(IRQ_LATENCIES)
                .cast::<u32>()
                .add(sample_index)
                .write(sample.cancellation_latency_cycles);
            addr_of_mut!(IRQ_ASSERTION_DELAYS)
                .cast::<u32>()
                .add(sample_index)
                .write(assertion_delay);
        }
    }
    disable_stack_guards();

    paint_stacks(STACK_PATTERN_B, STACK_PATTERN_B);
    let irq_stack_b_measurement = measure_once(&number, true);
    let irq_stack_b = inspect_stacks(irq_stack_b_measurement.baseline_psp, STACK_PATTERN_B, true);
    irq_allocations = irq_allocations.wrapping_add(irq_stack_b_measurement.allocations);
    irq_max = irq_max.max(irq_stack_b_measurement.cycles);
    irq_service_max = irq_service_max.max(irq_stack_b_measurement.irq_service_cycles);
    irq_observed += u32::from(irq_stack_b_measurement.cancellation_observed != 0);
    result_mismatches += u32::from(irq_stack_b_measurement.result_identity() != expected_result);

    let masked_stack_conservative = masked_stack_a
        .project_depth
        .max(masked_stack_b.project_depth);
    let irq_project_stack_conservative = irq_stack_a.project_depth.max(irq_stack_b.project_depth);
    let interrupt_stack_conservative = irq_stack_a.irq_depth.max(irq_stack_b.irq_depth);
    let guard_ok = masked_stack_a.guard_ok
        && masked_stack_b.guard_ok
        && irq_stack_a.guard_ok
        && irq_stack_b.guard_ok;
    let stack_certain = masked_stack_a.certain
        && masked_stack_b.certain
        && irq_stack_a.certain
        && irq_stack_b.certain;
    let expected_irq_observations = (WARMUPS + SAMPLES + 2) as u32;
    let failed = masked_max > CYCLE_LIMIT
        || masked_stack_conservative > STACK_LIMIT
        || irq_project_stack_conservative > STACK_LIMIT
        || masked_allocations != 0
        || irq_allocations != 0
        || irq_observed != expected_irq_observations
        || result_mismatches != 0
        || !guard_ok
        || !stack_certain;

    emit_case(
        index,
        family,
        text,
        expected_result,
        masked_cold,
        masked_stack_a,
        masked_stack_b_measurement,
        masked_stack_b,
        masked_max,
        masked_allocations,
        masked_stack_conservative,
        irq_cold,
        irq_stack_a,
        irq_stack_b_measurement,
        irq_stack_b,
        irq_max,
        irq_service_max,
        irq_allocations,
        irq_observed,
        expected_irq_observations,
        irq_project_stack_conservative,
        interrupt_stack_conservative,
        result_mismatches,
        guard_ok,
        stack_certain,
        failed,
    );
    failed
}

#[derive(Clone, Copy)]
struct StackObservation {
    project_depth: usize,
    irq_depth: usize,
    guard_ok: bool,
    certain: bool,
}

fn project_stack_base() -> *mut u8 {
    unsafe { addr_of_mut!(PROJECT_STACK.0).cast::<u8>() }
}

fn irq_stack_base() -> *mut u8 {
    unsafe { addr_of_mut!(IRQ_STACK.0).cast::<u8>() }
}

fn paint_stacks(project_pattern: u32, irq_pattern: u32) {
    disable_stack_guards();
    unsafe {
        paint_words(
            addr_of_mut!(PROJECT_STACK.0).cast(),
            PROJECT_GUARD_BYTES,
            GUARD_PATTERN,
        );
        paint_words(
            addr_of_mut!(PROJECT_STACK.0)
                .cast::<u8>()
                .add(PROJECT_GUARD_BYTES)
                .cast(),
            PROJECT_PAINT_BYTES,
            project_pattern,
        );
        paint_words(
            addr_of_mut!(IRQ_STACK.0).cast(),
            IRQ_GUARD_BYTES,
            GUARD_PATTERN,
        );
        paint_words(
            addr_of_mut!(IRQ_STACK.0)
                .cast::<u8>()
                .add(IRQ_GUARD_BYTES)
                .cast(),
            IRQ_PAINT_BYTES,
            irq_pattern,
        );
    }
}

unsafe fn paint_words(start: *mut u32, bytes: usize, pattern: u32) {
    for index in 0..bytes / size_of::<u32>() {
        unsafe {
            start.add(index).write_volatile(pattern);
        }
    }
}

fn inspect_stacks(baseline_psp: u32, pattern: u32, include_irq: bool) -> StackObservation {
    disable_stack_guards();
    let project_base = project_stack_base() as usize;
    let project_paint = project_base + PROJECT_GUARD_BYTES;
    let project_top = project_base + PROJECT_STACK_BYTES;
    let irq_base = irq_stack_base() as usize;
    let irq_paint = irq_base + IRQ_GUARD_BYTES;
    let irq_top = irq_base + IRQ_STACK_BYTES;
    let project_low = first_changed(project_paint, PROJECT_PAINT_BYTES, pattern);
    let irq_low = if include_irq {
        first_changed(irq_paint, IRQ_PAINT_BYTES, pattern)
    } else {
        None
    };
    let baseline = baseline_psp as usize;
    let project_depth = project_low
        .and_then(|low| baseline.checked_sub(low))
        .unwrap_or(PROJECT_PAINT_BYTES + 1);
    let irq_depth = irq_low.map_or(0, |low| irq_top - low);
    let guard_ok = words_match(project_base, PROJECT_GUARD_BYTES, GUARD_PATTERN)
        && words_match(irq_base, IRQ_GUARD_BYTES, GUARD_PATTERN);
    let certain = (project_paint..=project_top).contains(&baseline)
        && project_low.is_some()
        && (!include_irq || irq_low.is_some());
    StackObservation {
        project_depth,
        irq_depth,
        guard_ok,
        certain,
    }
}

fn first_changed(start: usize, bytes: usize, pattern: u32) -> Option<usize> {
    for offset in (0..bytes).step_by(size_of::<u32>()) {
        let value = unsafe { ((start + offset) as *const u32).read_volatile() };
        if value != pattern {
            return Some(start + offset);
        }
    }
    None
}

fn words_match(start: usize, bytes: usize, pattern: u32) -> bool {
    (0..bytes)
        .step_by(size_of::<u32>())
        .all(|offset| unsafe { ((start + offset) as *const u32).read_volatile() == pattern })
}

fn measure_once(number: &Number, irq_mode: bool) -> Measurement {
    enable_stack_guards();
    let result = measure_once_guarded(number, irq_mode);
    disable_stack_guards();
    result
}

fn measure_once_guarded(number: &Number, irq_mode: bool) -> Measurement {
    let project_base = project_stack_base();
    let irq_base = irq_stack_base();
    let context = InvokeContext {
        project_stack_top: unsafe { project_base.add(PROJECT_STACK_BYTES) },
        irq_stack_top: unsafe { irq_base.add(IRQ_STACK_BYTES) },
        irq_mode: u32::from(irq_mode),
    };
    let mut measurement = Measurement::ZERO;
    unsafe {
        invoke_on_dedicated_stacks(number, &mut measurement, &context);
    }
    measurement
}

#[allow(clippy::too_many_arguments)]
fn emit_case(
    index: usize,
    family: &str,
    text: &str,
    result: (u32, u64),
    masked_cold: Measurement,
    masked_stack_a: StackObservation,
    masked_stack_b_measurement: Measurement,
    masked_stack_b: StackObservation,
    masked_max: u32,
    masked_allocations: u32,
    masked_stack_conservative: usize,
    irq_cold: Measurement,
    irq_stack_a: StackObservation,
    irq_stack_b_measurement: Measurement,
    irq_stack_b: StackObservation,
    irq_max: u32,
    irq_service_max: u32,
    irq_allocations: u32,
    irq_observed: u32,
    expected_irq_observations: u32,
    irq_project_stack_conservative: usize,
    interrupt_stack_conservative: usize,
    result_mismatches: u32,
    guard_ok: bool,
    stack_certain: bool,
    failed: bool,
) {
    let mut output = Output::new();
    write!(
        output,
        "{{\"record\":\"case\",\"case_index\":{index},\"family\":\"{family}\",\"lexeme\":\"{text}\",\"length\":{},\"projection\":",
        text.len()
    )
    .unwrap();
    match result.0 {
        0 => write!(
            output,
            "{{\"kind\":\"finite\",\"bits\":\"0x{:016x}\"}}",
            result.1
        ),
        1 => output.write_str("{\"kind\":\"rejection\",\"value\":\"Limit\"}"),
        2 => output.write_str("{\"kind\":\"rejection\",\"value\":\"InvalidSchema\"}"),
        _ => output.write_str("{\"kind\":\"rejection\",\"value\":\"InvalidConfiguration\"}"),
    }
    .unwrap();
    write!(
        output,
        ",\"masked\":{{\"cold_cycles\":{},\"stack_pattern_a_depth_bytes\":{},\"stack_pattern_a_baseline_psp\":{},\"stack_pattern_b_cycles\":{},\"stack_pattern_b_depth_bytes\":{},\"stack_pattern_b_baseline_psp\":{},\"stack_watermark_conservative_depth_bytes\":{},\"max_cycles\":{},\"allocation_calls\":{},\"samples\":[",
        masked_cold.cycles,
        masked_stack_a.project_depth,
        masked_cold.baseline_psp,
        masked_stack_b_measurement.cycles,
        masked_stack_b.project_depth,
        masked_stack_b_measurement.baseline_psp,
        masked_stack_conservative,
        masked_max,
        masked_allocations,
    )
    .unwrap();
    write_sample_array(&mut output, addr_of!(MASKED_SAMPLES).cast());
    write!(
        output,
        "]}},\"irq_loaded\":{{\"cold_cycles\":{},\"cold_cancellation_assertion_delay_cycles\":{},\"cold_cancellation_latency_cycles\":{},\"stack_pattern_a_project_depth_bytes\":{},\"stack_pattern_a_interrupt_depth_bytes\":{},\"stack_pattern_b_cycles\":{},\"stack_pattern_b_cancellation_assertion_delay_cycles\":{},\"stack_pattern_b_cancellation_latency_cycles\":{},\"stack_pattern_b_project_depth_bytes\":{},\"stack_pattern_b_interrupt_depth_bytes\":{},\"project_stack_watermark_conservative_depth_bytes\":{},\"interrupt_stack_watermark_conservative_depth_bytes\":{},\"max_cycles\":{},\"irq_service_body_max_cycles\":{},\"allocation_calls\":{},\"cancellation_observed\":{},\"cancellation_expected\":{},\"samples\":[",
        irq_cold.cycles,
        irq_cold
            .cancellation_cycle
            .wrapping_sub(irq_cold.operation_entry_cycle),
        irq_cold.cancellation_latency_cycles,
        irq_stack_a.project_depth,
        irq_stack_a.irq_depth,
        irq_stack_b_measurement.cycles,
        irq_stack_b_measurement
            .cancellation_cycle
            .wrapping_sub(irq_stack_b_measurement.operation_entry_cycle),
        irq_stack_b_measurement.cancellation_latency_cycles,
        irq_stack_b.project_depth,
        irq_stack_b.irq_depth,
        irq_project_stack_conservative,
        interrupt_stack_conservative,
        irq_max,
        irq_service_max,
        irq_allocations,
        irq_observed,
        expected_irq_observations,
    )
    .unwrap();
    write_sample_array(&mut output, addr_of!(IRQ_SAMPLES).cast());
    output
        .write_str("],\"cancellation_assertion_delay_cycles\":[")
        .unwrap();
    write_sample_array(&mut output, addr_of!(IRQ_ASSERTION_DELAYS).cast());
    output
        .write_str("],\"cancellation_latency_cycles\":[")
        .unwrap();
    write_sample_array(&mut output, addr_of!(IRQ_LATENCIES).cast());
    writeln!(
        output,
        "]}},\"result_mismatches\":{result_mismatches},\"guard_ok\":{guard_ok},\"stack_watermark_certain\":{stack_certain},\"failed\":{failed}}}"
    )
    .unwrap();
    output.flush();
}

fn write_sample_array(output: &mut Output, samples: *const u32) {
    for index in 0..SAMPLES {
        if index != 0 {
            output.write_char(',').unwrap();
        }
        let value = unsafe { samples.add(index).read() };
        if value == u32::MAX {
            output.write_str("null").unwrap();
        } else {
            write!(output, "{value}").unwrap();
        }
    }
}

struct Output {
    length: usize,
}

impl Output {
    fn new() -> Self {
        Self { length: 0 }
    }

    fn flush(&mut self) {
        if self.length == 0 {
            return;
        }
        let buffer = addr_of!(OUTPUT_BUFFER).cast::<u8>();
        semihost_write(unsafe { core::slice::from_raw_parts(buffer, self.length) });
        self.length = 0;
    }
}

impl Write for Output {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let bytes = text.as_bytes();
        let mut offset = 0;
        while offset < bytes.len() {
            if self.length == OUTPUT_BYTES {
                self.flush();
            }
            let available = OUTPUT_BYTES - self.length;
            let copied = available.min(bytes.len() - offset);
            unsafe {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr().add(offset),
                    addr_of_mut!(OUTPUT_BUFFER).cast::<u8>().add(self.length),
                    copied,
                );
            }
            self.length += copied;
            offset += copied;
        }
        Ok(())
    }
}

fn configure_clock_168mhz() {
    unsafe {
        RCC_CR.write_volatile(RCC_CR.read_volatile() | (1 << 16));
        while RCC_CR.read_volatile() & (1 << 17) == 0 {}

        FLASH_ACR.write_volatile((1 << 10) | (1 << 9) | (1 << 8) | 5);
        RCC_PLLCFGR.write_volatile(8 | (336 << 6) | (1 << 22) | (7 << 24));
        RCC_CFGR.write_volatile((0b101 << 10) | (0b100 << 13));
        RCC_CR.write_volatile(RCC_CR.read_volatile() | (1 << 24));
        while RCC_CR.read_volatile() & (1 << 25) == 0 {}
        RCC_CFGR.write_volatile(RCC_CFGR.read_volatile() | 0b10);
        while RCC_CFGR.read_volatile() & (0b11 << 2) != (0b10 << 2) {}

        RCC_AHB1ENR.write_volatile(RCC_AHB1ENR.read_volatile() | 1);
        asm!("dsb", "isb", options(nomem, nostack, preserves_flags));
    }
}

fn configure_cycle_counter() {
    unsafe {
        SCB_DEMCR.write_volatile(SCB_DEMCR.read_volatile() | (1 << 24));
        DWT_CYCCNT.write_volatile(0);
        DWT_CTRL.write_volatile(DWT_CTRL.read_volatile() | 1);
    }
    assert_ne!(cycle_count(), cycle_count());
}

fn configure_tim2() {
    unsafe {
        RCC_APB1ENR.write_volatile(RCC_APB1ENR.read_volatile() | 1);
        TIM2_CR1.write_volatile(0);
        TIM2_DIER.write_volatile(1);
        TIM2_PSC.write_volatile(0);
        TIM2_ARR.write_volatile(32);
        TIM2_EGR.write_volatile(1);
        TIM2_SR.write_volatile(0);
        NVIC_IPR.add(28).write_volatile(0x80);
        NVIC_ICPR0.write_volatile(1 << 28);
        NVIC_ISER0.write_volatile(1 << 28);
    }
}

fn arm_cancellation_interrupt() {
    unsafe {
        TIM2_CR1.write_volatile(0);
        TIM2_SR.write_volatile(0);
        TIM2_CNT.write_volatile(0);
        TIM2_ARR.write_volatile(32);
        TIM2_EGR.write_volatile(1);
        TIM2_SR.write_volatile(0);
        NVIC_ICPR0.write_volatile(1 << 28);
        TIM2_CR1.write_volatile((1 << 3) | 1);
    }
}

fn disarm_cancellation_interrupt() {
    unsafe {
        TIM2_CR1.write_volatile(0);
        TIM2_SR.write_volatile(0);
        NVIC_ICPR0.write_volatile(1 << 28);
    }
}

fn enable_stack_guards() {
    let project_guard = project_stack_base() as u32;
    let irq_guard = irq_stack_base() as u32;
    assert_eq!(project_guard & 0xff, 0);
    assert_eq!(irq_guard & 0xff, 0);
    unsafe {
        MPU_CTRL.write_volatile(0);
        MPU_RNR.write_volatile(6);
        MPU_RBAR.write_volatile(project_guard);
        MPU_RASR.write_volatile((1 << 28) | (7 << 1) | 1);
        MPU_RNR.write_volatile(7);
        MPU_RBAR.write_volatile(irq_guard);
        MPU_RASR.write_volatile((1 << 28) | (7 << 1) | 1);
        SCB_SHCSR.write_volatile(SCB_SHCSR.read_volatile() | (1 << 16));
        MPU_CTRL.write_volatile((1 << 2) | 1);
        asm!("dsb", "isb", options(nomem, nostack, preserves_flags));
    }
}

fn disable_stack_guards() {
    unsafe {
        MPU_CTRL.write_volatile(0);
        asm!("dsb", "isb", options(nomem, nostack, preserves_flags));
    }
}

fn reset_flash_caches() {
    unsafe {
        let configured = FLASH_ACR.read_volatile();
        let disabled = configured & !((1 << 8) | (1 << 9) | (1 << 10));
        FLASH_ACR.write_volatile(disabled);
        FLASH_ACR.write_volatile(disabled | (1 << 11) | (1 << 12));
        FLASH_ACR.write_volatile(configured & !((1 << 11) | (1 << 12)));
        FLASH_ACR.write_volatile(configured);
        asm!("dsb", "isb", options(nomem, nostack, preserves_flags));
    }
}

fn cycle_count() -> u32 {
    unsafe { DWT_CYCCNT.read_volatile() }
}

fn read_register(register: *mut u32) -> u32 {
    unsafe { register.read_volatile() }
}

fn user_button_pressed() -> bool {
    unsafe { GPIOA_IDR.read_volatile() & 1 != 0 }
}

#[inline(never)]
#[unsafe(no_mangle)]
extern "C" fn wp100_coverage_probe_active() {
    black_box(());
}

fn coverage_loop() -> ! {
    const MIDPOINT: &str = "1.00000000000000011102230246251565404236316680908203125";
    let mut text = String::from(MIDPOINT);
    text.push_str(&"0".repeat(HARD_MAX - MIDPOINT.len()));
    let number: Number = serde_json::from_str(&text).expect("coverage Number");
    let limit = LexemeLimit::new(HARD_MAX).unwrap();
    loop {
        wp100_coverage_probe_active();
        let _ = black_box(project(&number, limit));
    }
}

unsafe extern "C" fn hard_fault() {
    semihost_write(b"{\"record\":\"fatal\",\"kind\":\"fault-or-stack-guard\"}\n");
    semihost_exit(2)
}

unsafe extern "C" fn default_handler() {
    semihost_write(b"{\"record\":\"fatal\",\"kind\":\"unexpected-exception\"}\n");
    semihost_exit(3)
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    unsafe {
        asm!("cpsid i", options(nomem, nostack, preserves_flags));
    }
    let mut output = Output::new();
    let _ = writeln!(
        output,
        "{{\"record\":\"fatal\",\"kind\":\"panic\",\"detail\":\"{}\"}}",
        JsonDisplay(info)
    );
    output.flush();
    semihost_exit(4)
}

struct JsonDisplay<'a, T>(&'a T);

impl<T: fmt::Display> fmt::Display for JsonDisplay<'_, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Escaper<'a, 'b>(&'a mut fmt::Formatter<'b>);
        impl Write for Escaper<'_, '_> {
            fn write_str(&mut self, value: &str) -> fmt::Result {
                for character in value.chars() {
                    match character {
                        '"' => self.0.write_str("\\\""),
                        '\\' => self.0.write_str("\\\\"),
                        '\n' | '\r' | '\t' => self.0.write_char(' '),
                        c if c.is_control() => self.0.write_char('?'),
                        c => self.0.write_char(c),
                    }?;
                }
                Ok(())
            }
        }
        write!(Escaper(formatter), "{}", self.0)
    }
}

fn semihost_write(bytes: &[u8]) {
    let block = [1usize, bytes.as_ptr() as usize, bytes.len()];
    let _ = unsafe { semihost_call(0x05, block.as_ptr() as usize) };
}

fn semihost_exit(status: u32) -> ! {
    let reason = if status == 0 {
        0x20026usize
    } else {
        0x20023usize
    };
    let block = [reason, status as usize];
    let _ = unsafe { semihost_call(0x20, block.as_ptr() as usize) };
    loop {
        unsafe {
            asm!("bkpt #0", options(nomem, nostack));
        }
    }
}

unsafe fn semihost_call(operation: usize, parameter: usize) -> usize {
    let mut result = operation;
    unsafe {
        asm!(
            "bkpt #0xab",
            inout("r0") result,
            in("r1") parameter,
            options(nostack)
        );
    }
    result
}
