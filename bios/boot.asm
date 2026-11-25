; SPDX-License-Identifier: CC0-1.0
bits 16
cpu 186
org 0

%assign IPL_SEGMENT (0x10000 - IPL_SIZE / 16)
; Borrow scratch space just below the incoming stack instead of leaving
; instructions in the low application RAM that cartridges expect to be zero.
%define HANDOFF_RAM 0x1ff0

boot:
    cli
    cld
    xor ax, ax
    mov ss, ax
    mov sp, 0x2000
    mov es, ax

    ; Keep the existing display cadence and silent audio defaults at entry.
%if IPL_SIZE == 8192
    out 0x60, al
%endif
    mov al, 158
    out 0x16, al
%if !CRYSTAL
    mov al, 155
    out 0x17, al
%endif
    mov al, 3
    out 0x9e, al

    ; Leave the LCD awake like the hardware boot ROM after its splash screen.
    ; Some cartridges never write LCD control and rely on this handoff state.
    mov al, 1
    out 0x14, al

    ; The mapper masks the linear bank to its own width. The two ROM windows
    ; also retain the extended high bits supplied by the mapper reset state.
    mov al, 0xff
    out 0xc0, al
    out 0xc2, al
    out 0xc3, al

    ; Both color consoles are in mono command mode at cartridge entry.
    mov ax, 0x0130
    out 0xbc, ax
    mov al, 0x40
    out 0xbe, al

    ; The 16-bit WEN transfer completes while the CPU copies the stub.
    push cs
    pop ds
    mov si, handoff
    mov di, HANDOFF_RAM
    mov cx, handoff_end - handoff
    rep movsb
    xor si, si
    xor di, di
    xor ax, ax
    out 0xbc, ax

    ; ROM0 exposes the final cartridge bank independently of the IPL overlay.
    mov ax, 0x2000
    mov ds, ax
    test byte [0xfff9], 0x80
    jnz .configuration
    mov al, 0x80
    out 0xbe, al
.configuration:
    mov al, [0xfffc]
    and al, 0x0c
    or al, 1
    push cs
    pop ds
    jmp 0:HANDOFF_RAM

handoff:
    out 0xa0, al
    xor ax, ax
    push ax
    popf
    jmp 0xffff:0
handoff_end:

times IPL_SIZE - 16 - ($ - $$) db 0xff
    jmp IPL_SEGMENT:boot
times IPL_SIZE - ($ - $$) db 0xff
