; SPDX-License-Identifier: CC0-1.0
; Original Issue #29 input/presentation probe, no external code or assets.
bits 16
cpu 186
org 0
start:
    cli
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,0x3000
    cld
    mov di,0x2000
    mov cx,8
    rep stosw                   ; tile 0: white
    mov ax,0x00ff
    mov cx,8
    rep stosw                   ; tile 1: black
    mov ax,0xff00
    mov cx,8
    rep stosw                   ; tile 2: gray 170
    mov ax,0xffff
    mov cx,8
    rep stosw                   ; tile 3: gray 85
    xor ax,ax
    mov cx,8
    rep stosw                   ; tile 4: only bottom-right pixel black
    mov byte [0x204e],1
    mov word [0],1              ; native top-left tile
    mov word [54],2             ; native top-right tile
    mov word [1088],3           ; native bottom-left tile
    mov word [1142],4           ; native bottom-right tile
    mov ax,0x3210
    out 0x20,ax
    mov al,0xf0
    out 0x1c,al
    mov al,0xa5
    out 0x1d,al
    xor al,al
    out 7,al
    mov al,1
    out 0,al
    out 0x14,al
    mov ax,0x1000
    mov es,ax                  ; SRAM mirrors the actual CPU IN results
scan:
    xor si,si
    mov bx,128                 ; tilemap row 2; eight bit tiles per selection
.row:
    mov ax,si
    shl al,4                   ; selects 00,10,20,30,40,50,60,70
    out 0xb5,al
    nop
    nop
    in al,0xb5
    mov [0x400+si],al
    mov [es:si],al
    mov dl,al
    mov di,bx
    mov cx,8
.bit:
    xor ax,ax
    shr dl,1
    adc ax,0                   ; tile 1 for set bit, tile 0 for clear bit
    mov [di],ax
    add di,2
    loop .bit
    add bx,64
    inc si
    cmp si,8
    jne .row
    jmp scan
