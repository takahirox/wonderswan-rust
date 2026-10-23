; SPDX-License-Identifier: CC0-1.0
; Original Issue #33 incoming SP/DS probe. No stack or DS-relative access
; precedes capture; establish ES explicitly instead of assuming any segment.
bits 16
cpu 186
org 0
start:
    xor ax,ax
    mov es,ax
    mov [es:0x400],sp
    mov ax,ds
    mov [es:0x402],ax
    ; Only after both captures replace stack and data segments.
    cli
    xor ax,ax
    mov ss,ax
    mov sp,0x3000
    mov ds,ax
    cld
    in al,0xa0
    xor ah,ah
    mov [0x404],ax
    mov ax,cs
    mov [0x406],ax
    mov word [0x408],0x0bad
    cmp word [0x400],0x2000
    jne save
    test byte [0x404],2
    jnz color
    cmp word [0x402],0xff00
    jne save
    jmp passed
color:
    cmp word [0x402],0xfe00
    jne save
passed:
    mov word [0x408],0x600d
save:
    ; Normal common/extended initial SRAM bank 0. Copy CPU-captured values
    ; to real cartridge save bytes so the production player can observe them.
    mov ax,0x1000
    mov es,ax
    mov ax,[es:0]
    cmp ax,0xffff
    jne .increment
    xor ax,ax
.increment:
    inc ax
    mov [es:0],ax
    mov ax,[0x400]
    mov [es:2],ax
    mov ax,[0x402]
    mov [es:4],ax
    mov ax,[0x408]
    mov [es:6],ax
    mov ax,[0x404]
    mov [es:8],ax
    mov ax,[0x406]
    mov [es:10],ax
    ; Distinct stale values: every subsequent handoff must replace both.
    mov sp,0x3456
    mov ax,0x1234
    mov ds,ax
    hlt
