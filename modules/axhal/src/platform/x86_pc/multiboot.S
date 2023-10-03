# Bootstrapping from 32-bit with the Multiboot specification.
# See https://www.gnu.org/software/grub/manual/multiboot/multiboot.html

.section .text.boot
.code32
.global _start
_start:
    mov     edi, eax        # arg1: magic: 0x2BADB002
    mov     esi, ebx        # arg2: multiboot info

# ---------------------------------------------------------------------------
    # 打开蜂鸣器
    mov al, 0b10110011   # 控制字节：bit 7-6=11 (3)，bit 5=1 (使能蜂鸣器)
    out 0x61, al

    # 设置频率
    mov al, 0b00100000   # 控制字节：bit 7-5=000 (0) 表示选择低四位
    out 0x43, al

    mov al, 500 & 0xFF   # 将频率低字节写入数据端口(0x42)
    out 0x42, al

    mov al, 500 >> 8      # 将频率高字节写入数据端口(0x42)
    out 0x42, al

    # 延时一段时间
    mov cx, 65535
delay_loop:
    dec cx
    cmp cx, 0
    jne delay_loop

    # 关闭蜂鸣器
    mov al, 0x90            # 设定8254计数器2为二进制、方式3、读写低字节/高字节、计数器2。
    out 0x43, al            # 发送命令字节到控制字寄存器。

    in al, 0x61             # 从端口0x61（8255 PPI的Port B）读取当前值
    and al, 0xFC            # 操作位2和位1来关闭扬声器。
    out 0x61, al            # 写回新的值以实际应用更改。
# ---------------------------------------------------------------------------

    # 设置字符属性（颜色）
    mov ah, 0x0e        # 存储子功能号到寄存器 ah (0x0e 是用于 teletype 输出的功能)
    xor bh, bh          # 将寄存器 bh 清零 (0 是默认的显示页)

    mov al, 'A'         # 存储要显示的字符到寄存器 al
    int 0x10            # 使用 BIOS 中断 0x10 来执行显示操作

    mov al, 'r'         # 存储要显示的字符到寄存器 al
    int 0x10            # 使用 BIOS 中断 0x10 来执行显示操作

    mov al, 'c'         # 存储要显示的字符到寄存器 al
    int 0x10            # 使用 BIOS 中断 0x10 来执行显示操作

    mov al, 'e'         # 存储要显示的字符到寄存器 al
    int 0x10            # 使用 BIOS 中断 0x10 来执行显示操作

    mov al, 'O'         # 存储要显示的字符到寄存器 al
    int 0x10            # 使用 BIOS 中断 0x10 来执行显示操作

    mov al, 'S'         # 存储要显示的字符到寄存器 al
    int 0x10            # 使用 BIOS 中断 0x10 来执行显示操作

    mov al, '#'         # 存储要显示的字符到寄存器 al
    int 0x10            # 使用 BIOS 中断 0x10 来执行显示操作

    #mov sp, 0xC000
    #mov al, '!'
    #call print_char

    jmp     bsp_entry32

print_char:
    xor bh, bh
    mov ah, 0x0e
    mov al, '@'
    int 0x10
    ret

# 字符串
hello_message:
    .asciz "Hello, World!\n"

# print a string
# si: points at zero-terminated String
print:
    pushf
    cld
.loop:
    lodsb
    test al, al
    jz .done
    call print_char
    jmp .loop
.done:
    popf
    ret

.balign 4
.type multiboot_header, STT_OBJECT
multiboot_header:
    .int    {mb_hdr_magic}                      # magic: 0x1BADB002
    .int    {mb_hdr_flags}                      # flags
    .int    -({mb_hdr_magic} + {mb_hdr_flags})  # checksum
    .int    multiboot_header - {offset}         # header_addr
    .int    _skernel - {offset}                 # load_addr
    .int    _edata - {offset}                   # load_end
    .int    _ebss - {offset}                    # bss_end_addr
    .int    _start - {offset}                   # entry_addr

# Common code in 32-bit, prepare states to enter 64-bit.
.macro ENTRY32_COMMON
    # set data segment selectors
    mov     ax, 0x18
    mov     ss, ax
    mov     ds, ax
    mov     es, ax
    mov     fs, ax
    mov     gs, ax

    # set PAE, PGE bit in CR4
    mov     eax, {cr4}
    mov     cr4, eax

    # load the temporary page table
    lea     eax, [.Ltmp_pml4 - {offset}]
    mov     cr3, eax

    # set LME, NXE bit in IA32_EFER
    mov     ecx, {efer_msr}
    mov     edx, 0
    mov     eax, {efer}
    wrmsr

    # set protected mode, write protect, paging bit in CR0
    mov     eax, {cr0}
    mov     cr0, eax
.endm

# Common code in 64-bit
.macro ENTRY64_COMMON
    # clear segment selectors
    xor     ax, ax
    mov     ss, ax
    mov     ds, ax
    mov     es, ax
    mov     fs, ax
    mov     gs, ax
.endm

.code32
bsp_entry32:
    lgdt    [.Ltmp_gdt_desc - {offset}]             # load the temporary GDT
    ENTRY32_COMMON
    ljmp    0x10, offset bsp_entry64 - {offset}    # 0x10 is code64 segment

.code32
.global ap_entry32
ap_entry32:
    ENTRY32_COMMON
    ljmp    0x10, offset ap_entry64 - {offset}     # 0x10 is code64 segment

.code64
bsp_entry64:
    ENTRY64_COMMON

    # set RSP to boot stack
    movabs  rsp, offset {boot_stack}
    add     rsp, {boot_stack_size}

    # call rust_entry(magic, mbi)
    movabs  rax, offset {entry}
    call    rax
    jmp     .Lhlt

.code64
ap_entry64:
    ENTRY64_COMMON

    # set RSP to high address (already set in ap_start.S)
    mov     rax, {offset}
    add     rsp, rax

    # call rust_entry_secondary(magic)
    mov     rdi, {mb_magic}
    movabs  rax, offset {entry_secondary}
    call    rax
    jmp     .Lhlt

.Lhlt:
    hlt
    jmp     .Lhlt

.section .rodata
.balign 8
.Ltmp_gdt_desc:
    .short  .Ltmp_gdt_end - .Ltmp_gdt - 1   # limit
    .long   .Ltmp_gdt - {offset}            # base

.section .data
.balign 16
.Ltmp_gdt:
    .quad 0x0000000000000000    # 0x00: null
    .quad 0x00cf9b000000ffff    # 0x08: code segment (base=0, limit=0xfffff, type=32bit code exec/read, DPL=0, 4k)
    .quad 0x00af9b000000ffff    # 0x10: code segment (base=0, limit=0xfffff, type=64bit code exec/read, DPL=0, 4k)
    .quad 0x00cf93000000ffff    # 0x18: data segment (base=0, limit=0xfffff, type=32bit data read/write, DPL=0, 4k)
.Ltmp_gdt_end:

.balign 4096
.Ltmp_pml4:
    # 0x0000_0000 ~ 0xffff_ffff
    .quad .Ltmp_pdpt_low - {offset} + 0x3   # PRESENT | WRITABLE | paddr(tmp_pdpt)
    .zero 8 * 510
    # 0xffff_ff80_0000_0000 ~ 0xffff_ff80_ffff_ffff
    .quad .Ltmp_pdpt_high - {offset} + 0x3  # PRESENT | WRITABLE | paddr(tmp_pdpt)

# FIXME: may not work on macOS using hvf as the CPU does not support 1GB page (pdpe1gb)
.Ltmp_pdpt_low:
    .quad 0x0000 | 0x83         # PRESENT | WRITABLE | HUGE_PAGE | paddr(0x0)
    .quad 0x40000000 | 0x83     # PRESENT | WRITABLE | HUGE_PAGE | paddr(0x4000_0000)
    .quad 0x80000000 | 0x83     # PRESENT | WRITABLE | HUGE_PAGE | paddr(0x8000_0000)
    .quad 0xc0000000 | 0x83     # PRESENT | WRITABLE | HUGE_PAGE | paddr(0xc000_0000)
    .zero 8 * 508

.Ltmp_pdpt_high:
    .quad 0x0000 | 0x83         # PRESENT | WRITABLE | HUGE_PAGE | paddr(0x0)
    .quad 0x40000000 | 0x83     # PRESENT | WRITABLE | HUGE_PAGE | paddr(0x4000_0000)
    .quad 0x80000000 | 0x83     # PRESENT | WRITABLE | HUGE_PAGE | paddr(0x8000_0000)
    .quad 0xc0000000 | 0x83     # PRESENT | WRITABLE | HUGE_PAGE | paddr(0xc000_0000)
    .zero 8 * 508
