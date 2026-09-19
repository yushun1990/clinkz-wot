ENTRY(Reset);

MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  SRAM  : ORIGIN = 0x20000000, LENGTH = 128K
  CCM   : ORIGIN = 0x10000000, LENGTH = 64K
}

_stack_top = ORIGIN(CCM) + LENGTH(CCM);

SECTIONS
{
  .vector_table ORIGIN(FLASH) : ALIGN(256)
  {
    KEEP(*(.vector_table));
  } > FLASH

  .text : ALIGN(4)
  {
    *(.text .text.*);
    *(.rodata .rodata.*);
  } > FLASH

  .ARM.exidx : ALIGN(4)
  {
    *(.ARM.exidx .ARM.exidx.*);
  } > FLASH

  .data : ALIGN(8)
  {
    _sdata = .;
    *(.data .data.*);
    _edata = .;
  } > SRAM AT > FLASH
  _sidata = LOADADDR(.data);

  .bss (NOLOAD) : ALIGN(8)
  {
    _sbss = .;
    *(.bss .bss.*);
    *(COMMON);
    _ebss = .;
  } > SRAM

  .uninit (NOLOAD) : ALIGN(8)
  {
    *(.uninit .uninit.*);
  } > SRAM

  /DISCARD/ :
  {
    *(.ARM.exidx.* .ARM.extab.*);
  }
}

ASSERT((_ebss - ORIGIN(SRAM)) <= LENGTH(SRAM), "SRAM sections overflow");
