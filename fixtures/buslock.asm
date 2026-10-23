; SPDX-License-Identifier: CC0-1.0
; Original Issue #35 probe. F0 bytes are explicit because NASM's x86 LOCK
; target restrictions do not describe V30MZ BUSLOCK. No imported assets/code.
bits 16
cpu 186
org 0

%macro buslock 0
    db 0xf0
%endmacro
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
    ; Retained count proves a new CPU run reached the results on restart.
    mov ax,0x1000
    mov es,ax
    mov ax,[es:0x18]
    cmp ax,0xffff
    jne .count
    xor ax,ax
.count:
    inc ax
    mov [0x418],ax
    xor ax,ax
    mov es,ax

    mov ax,0x1234
    buslock
    mov [0x900],ax
    xor ax,ax
    buslock
    mov ax,[0x900]
    cmp ax,0x1234
    jne failed
    mov bx,0x900
    buslock
    mov byte [bx+2],0x5a
    buslock
    mov al,[bx+2]
    xor ah,ah
    mov [0x404],ax
    cmp ax,0x5a
    jne failed
    buslock
    mov word [bx+4],0xabcd
    buslock
    add word [bx+4],1
    buslock
    mov ax,[bx+4]
    mov [0x402],ax
    buslock
    cmp word [bx+4],0xabce
    buslock
    pushf
    buslock
    pop ax
    mov [0x406],ax
    cmp ax,0xf046
    jne failed
    mov ax,0x1234
    buslock
    push ax
    xor ax,ax
    buslock
    pop ax
    mov [0x408],ax
    cmp ax,0x1234
    jne failed
    cmp sp,0x3000
    jne failed

    mov al,0x10
    buslock
    out 0x20,al
    xor al,al
    buslock
    in al,0x20
    cmp al,0x10
    jne failed
    mov ax,0x3210
    buslock
    out 0x20,ax
    xor ax,ax
    buslock
    in ax,0x20
    mov [0x40a],ax
    cmp ax,0x3210
    jne failed
    mov dx,0x20
    mov al,0x54
    buslock
    out dx,al
    xor al,al
    buslock
    in al,dx
    cmp al,0x54
    jne failed
    mov ax,0x7654
    buslock
    out dx,ax
    xor ax,ax
    buslock
    in ax,dx
    mov [0x40c],ax
    cmp ax,0x7654
    jne failed

    mov ax,0x100
    mov es,ax
    mov word [es:0x80],0xbeef
    mov word [0x80],0xdead
    ; Seven redundant/mixed prefixes: last segment wins (ES).
    db 0xf0,0x3e,0xf0,0x36,0xf0,0xf0,0x26
    mov ax,[0x80]
    mov [0x40e],ax
    cmp ax,0xbeef
    jne failed
    xor ax,ax
    mov es,ax
    mov word [0x920],0x1234
    mov word [0x922],0x5678
    mov si,0x920
    mov di,0x930
    mov cx,2
    db 0xf0,0x26,0xf0,0x3e,0xf0,0xf0,0xf3
    movsw
    mov ax,[0x930]
    mov [0x410],ax
    cmp ax,0x1234
    jne failed
    mov ax,[0x932]
    mov [0x412],ax
    cmp ax,0x5678
    jne failed
    cmp cx,0
    jne failed
    buslock
    nop
    mov ax,0x1234
    buslock
    add ax,ax
    mov [0x414],ax
    cmp ax,0x2468
    jne failed
    mov ax,cs
    mov [0x416],ax
    cmp ax,0xf000
    jne failed
    mov word [0x400],0x600d
    jmp display
failed:
    mov word [0x400],0x0bad
display:
    ; Copy actual CPU observations into normal cartridge SRAM for the browser.
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,13
    rep movsw
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
