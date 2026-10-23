; SPDX-License-Identifier: CC0-1.0
; Original Issue #41 cartridge. No firmware, imported code or assets.
; Literal db avoids NASM interpreting FPO1 as x87 ModR/M instructions.
bits 16
cpu 186
org 0

%macro put 2
    mov al,%2
    out %1,al
%endmacro

start:
    cli
    cld
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,0x3000
    mov di,0x400
    mov cx,22
    rep stosw                   ; clear volatile results on every boot
    mov ax,0x1000
    mov es,ax
    mov ax,[es:0x2a]             ; retained count, never a saved pass marker
    cmp ax,0xffff
    jne .count
    xor ax,ax
.count:
    inc ax
    mov [0x42a],ax
    mov word [es:0],0           ; invalidate saved completion before probing
    xor ax,ax
    mov es,ax
    mov bx,0x900
    mov bp,0x910
    mov si,0x920
    mov di,0x930
    mov word [0x900],0xbeef

    mov ax,0x1234
    push word 0xf003
    popf
    db 0x9b                     ; POLL preserves carry and all flags
    pushf
    pop dx
    mov [0x404],dx
    adc ax,0                    ; discriminates continuation and carry
    mov [0x402],ax
    cmp ax,0x1235
    jne failed
    cmp dx,0xf003
    jne failed

%macro fpo 4
    mov ax,%3
    push word 0xf003
    popf
    %if %1 == 0xdf
        ; Seven supported redundant/segment prefixes, ineffective BUSLOCK.
        db 0xf0,0x26,0x2e,0x36,0x3e,0xf0,0xf0
    %endif
    db %1,%2
    pushf                       ; this must be the next opcode, not displacement
    pop dx
    mov [%4+2],dx
    adc ax,0
    mov [%4],ax
    cmp ax,%3+1
    jne failed
    cmp dx,0xf003
    jne failed
%endmacro
    fpo 0xd8,0x06,0x2200,0x406   ; apparent direct address
    fpo 0xd9,0x46,0x2201,0x40a   ; apparent disp8
    fpo 0xda,0x86,0x2202,0x40e   ; apparent disp16
    fpo 0xdb,0x00,0x2203,0x412   ; apparent memory operand
    fpo 0xdc,0xf0,0x2204,0x416   ; swallowed BUSLOCK
    fpo 0xdd,0xf3,0x2205,0x41a   ; swallowed REP must not reject PUSHF
    fpo 0xde,0x26,0x2206,0x41e   ; swallowed segment override
    fpo 0xdf,0xc0,0x2207,0x422
    cmp bx,0x900
    jne failed
    cmp bp,0x910
    jne failed
    cmp si,0x920
    jne failed
    cmp di,0x930
    jne failed
    cmp sp,0x3000
    jne failed
    mov ax,[0x900]
    mov [0x426],ax
    cmp ax,0xbeef
    jne failed
    mov ax,cs
    mov [0x428],ax
    cmp ax,0xf000
    jne failed
    mov word [0x400],0x600d
    jmp display
failed:
    mov word [0x400],0x0bad
display:
    cld
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,22
    rep movsw                   ; mirror actual observations to normal SRAM
    xor ax,ax
    mov es,ax
    in al,0xa0
    test al,2
    jz .mono
    put 0x60,0x80
    mov bx,0x00f0
    cmp word [0x400],0x600d
    je .color
    mov bx,0x0f00
.color:
    mov [0xfe02],bx
    jmp .tile
.mono:
    mov ax,0x7654
    cmp word [0x400],0x600d
    je .gray
    mov ax,0x7653
.gray:
    out 0x20,ax
    put 0x1c,0x10
    put 0x1d,0x32
    put 0x1e,0x54
    put 0x1f,0x76
.tile:
    mov di,0x2000
    mov ax,0x00ff
    mov cx,8
    rep stosw
    put 7,0
    put 0,1
    put 0x14,1
    hlt
