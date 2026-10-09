/* QEMU MPS2 AN386: code in SSRAM1, data and stack in SSRAM2/3. */
MEMORY {
    CODE (rx) : ORIGIN = 0x00000000, LENGTH = 4M
    RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 4M
}
ENTRY(Reset)
SECTIONS {
    .vector_table : { KEEP(*(.vector_table)) } > CODE
    .text : { *(.text .text.*) } > CODE
    .rodata : { *(.rodata .rodata.*) } > CODE
    .ARM.exidx : { *(.ARM.exidx*) } > CODE
    .data : ALIGN(8) {
        _sdata = .;
        *(.data .data.*)
        . = ALIGN(8);
        _edata = .;
    } > RAM AT > CODE
    _sidata = LOADADDR(.data);
    .bss (NOLOAD) : ALIGN(64) {
        _sbss = .;
        *(.bss .bss.*) *(COMMON)
        . = ALIGN(8);
        _ebss = .;
    } > RAM
    _stack_start = ORIGIN(RAM) + LENGTH(RAM);
    /DISCARD/ : { *(.eh_frame*) }
}
