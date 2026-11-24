; SPDX-License-Identifier: CC0-1.0
; Original incoming SP/DS and IPL lockout probe (Issues #33 and #140).
; No stack or DS-relative access precedes capture.
bits 16
cpu 186
org 0
start:
    xor ax,ax
    mov es,ax
    mov ax,[es:0x1ff0]
    mov [es:0x40a],ax         ; Capture the IPL stub before using the stack.
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
    cmp word [0x40a],0xa0e6   ; OUT A0,AL in the RAM-resident IPL stub
    jne save
    mov al,[0x404]
    and al,0xfe              ; Attempt to reopen the IPL.
    out 0xa0,al
    in al,0xa0
    cmp al,[0x404]
    jne save
    test al,1
    jz save
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
    mov ax,[0x40a]
    mov [es:12],ax
    ; Distinct stale values: every subsequent handoff must replace both.
    mov sp,0x3456
    mov ax,0x1234
    mov ds,ax
    hlt
