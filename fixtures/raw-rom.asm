; SPDX-License-Identifier: CC0-1.0
; Original Issue #25 raw 3/6 MiB startup, bank and DMA probe.
; build.py supplies FIRST_BANK and independently generated bank markers.
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
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,0x3000
    cld
    in al,0xa0
    and ax,0x0083
    mov bp,ax
    ; FFFF:0000 must reach the reset jump in the final populated bank.
    mov word [0x408],0xf000
    put 0xc2,FIRST_BANK
    put 0xc3,FIRST_BANK+1
    mov ax,0x2000
    mov es,ax
    cmp word [es:0],0xa55a         ; first two original bytes
    jne failed
    cmp byte [es:0xffff],0xa5      ; last byte before the bank boundary
    jne failed
    mov ax,[es:0]
    mov [0x40a],ax
    mov ax,0x3000
    mov es,ax
    cmp word [es:0],0xa45b         ; next raw bank, distinct marker
    jne failed
    put 0xc2,FIRST_BANK-1
    mov ax,0x2000
    mov es,ax
    cmp word [es:0],0xffff         ; explicit synthetic leading-fill policy
    jne failed
    put 0xc2,FIRST_BANK+16
    cmp word [es:0],0xb54a         ; interior raw bank 16
    jne failed
    put 0xc2,FIRST_BANK
%if EXTENDED
    ; Low-byte aliases and high bits must retain the same rounded-space map.
    put 0xd2,FIRST_BANK
    put 0xd3,0xff
    cmp word [es:0],0xa55a
    jne failed
    put 0xd0,FIRST_BANK
    put 0xd1,0xff
    put 0xce,1
    mov ax,0x1000
    mov es,ax
    cmp word [es:0],0xa55a
    jne failed
    put 0xce,0
%endif
    ; Increment an independent SRAM boot counter. Console EEPROM is untouched.
    put 0xc1,0
%if EXTENDED
    put 0xd1,0
%endif
    mov ax,0x1000
    mov es,ax
    mov al,[es:0]
    cmp al,0xff
    jne .increment
    xor al,al
.increment:
    inc al
    mov [es:0],al
    mov [0x406],al
    mov ax,bp
    test al,2
    jnz color
    mov word [0x400],0x600d
    mov bx,0x7654
    jmp display
color:
    put 0x60,0x80
    put 0xa0,4
    ; GDMA crosses from C2:FFFE into C3:0000; both windows are populated.
    put 0x40,0xfe
    put 0x41,0xff
    put 0x42,2
    put 0x44,0
    put 0x45,0x10
    put 0x46,4
    put 0x47,0
    put 0x48,0x80
    check 0x40,2
    check 0x41,0
    check 0x42,3
    check 0x44,4
    check 0x45,0x10
    check 0x46,0
    check 0x48,0
    cmp word [0x1000],0xa55a
    jne failed
    cmp word [0x1002],0xa45b
    jne failed
    mov word [0x402],1
    ; SDMA uses the same bank boundary, slow live A0 timing, Channel 2.
    put 0xa0,12
    put 0x4a,0xfe
    put 0x4b,0xff
    put 0x4c,2
    put 0x4e,4
    put 0x4f,0
    put 0x50,0
    put 0x52,0x80
.wait:
    in al,0x52
    test al,0x80
    jnz .wait
    check 0x89,0xa4
    check 0x4a,2
    check 0x4b,0
    check 0x4c,3
    check 0x4e,0
    check 0x4f,0
    check 0x50,0
    ; Slow ROM remains ineligible for GDMA; pending length must survive.
    put 0x46,2
    put 0x48,0x80
    check 0x46,2
    check 0x48,0
    mov word [0x404],1
    mov word [0x400],0x600d
    mov bx,0x00f0
    jmp display
failed:
    mov word [0x400],0x0bad
    mov bx,0x0f00
    mov ax,bp
    test al,2
    jnz display
    mov bx,0x7653
display:
    mov ax,bp
    test al,2
    jz .mono
    mov [0xfe02],bx
    jmp .tiles
.mono:
    mov ax,bx
    out 0x20,ax
    put 0x1c,0x10
    put 0x1d,0x32
    put 0x1e,0x54
    put 0x1f,0x76
.tiles:
    xor ax,ax
    mov es,ax
    mov di,0x2000
    mov ax,0x00ff
    mov cx,8
    rep stosw
    put 7,0
    put 0,1
    put 0x14,1
    hlt
