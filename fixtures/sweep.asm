; SPDX-License-Identifier: CC0-1.0
; Original Issue #31 CPU probe; no imported code, firmware or sound assets.
bits 16
cpu 186
org 0

%macro put 2
    mov al,%2
    out %1,al
%endmacro
%macro check 2
    in al,%1
    cmp al,%2
    jne failed
%endmacro

start:
    cli
    cld
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,0x3000
    in al,0xa0
    mov bp,ax
    test al,2
    jz .mono
    put 0x60,0x80
.mono:
    ; Emulator readback/reset policy: must reset even if last run left FAST set.
    check 0x95,0
    xor ah,ah
    mov [0x40a],ax
    put 0x95,0
    check 0x95,0
    mov word [0x402],1

    put 0x8c,1
    put 0x8d,0
    mov ax,0x100
    out 0x84,ax
    put 0x90,0x44
    put 0x95,2
    check 0x95,2
    call short_delay
    put 0x90,0                ; freeze before word IN snapshot
    in ax,0x84
    sub ax,0x100
    mov [0x404],ax
    cmp ax,32                 ; well beyond ordinary sweep; bounded below wrap
    jb failed
    cmp ax,1536
    ja failed

    ; Clearing restores ordinary operation: unchanged during a short delay,
    ; then exactly one step over forty scanlines (10240 clocks plus overhead).
    put 0x95,0
    check 0x95,0
    put 0x8d,0                ; reset timer after mode change
    mov ax,0x100
    out 0x84,ax
    put 0x90,0x44
    call short_delay
    in ax,0x84
    mov [0x406],ax
    cmp ax,0x100
    jne failed
    in al,2
    mov dl,al
    mov cx,40
.line:
    in al,2
    cmp al,dl
    je .line
    mov dl,al
    loop .line
    put 0x90,0
    in ax,0x84
    mov [0x408],ax
    cmp ax,0x101
    jne failed

    ; Leave FAST set, with sweep disabled, so restart checks actual control reset.
    put 0x95,2
    check 0x95,2
    xor ah,ah
    mov [0x40c],ax
    ; Independently generated channel-1 square, period 2048 clocks = 1500 Hz.
    mov di,0x1000
    xor ax,ax
    mov cx,4
    rep stosw
    mov ax,0xffff
    mov cx,4
    rep stosw
    put 0x8f,0x40
    mov ax,0x7c0
    out 0x80,ax
    put 0x88,0xff
    put 0x91,8
    put 0x90,1
    mov word [0x400],0x600d
    mov bx,0x00f0
    mov ax,bp
    test al,2
    jnz display
    mov bx,0x7654              ; monochrome pass shade 170
    jmp display
failed:
    mov word [0x400],0x0bad
    mov bx,0x0f00
    mov ax,bp
    test al,2
    jnz display
    mov bx,0x7653              ; monochrome failure shade 187
display:
    ; CPU-produced observations survive in browser-local SRAM for assertions.
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,7
    rep movsw
    xor ax,ax
    mov es,ax
    mov ax,bp
    test al,2
    jz .gray
    mov [0xfe02],bx
    jmp .tile
.gray:
    mov ax,bx
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
short_delay:
    mov cx,80
.wait:
    loop .wait
    ret
