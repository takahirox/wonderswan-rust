; SPDX-License-Identifier: CC0-1.0
; Original GDMA probe for Issue #13. No third-party code, assets or firmware.
bits 16
cpu 186
org 0

%macro put_port 2
    mov al,%2
    out %1,al
%endmacro
%macro check_port 2
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
    test al,2
    jnz color
    mov word [0x400],0x4d4f        ; mono: skipped, no GDMA port accessed
    hlt

color:
    put_port 0x60,0x80
    mov word [0x1000],0x1122
    mov word [0x1002],0x3344
    mov word [0x1004],0x5566
    ; Forward copy. Odd low bits/high reserved bits must be masked.
    put_port 0x40,1
    put_port 0x41,0x10
    put_port 0x42,0xf0
    put_port 0x44,1
    put_port 0x45,0x11
    put_port 0x46,7
    put_port 0x47,0
    put_port 0x48,0xbf
    ; The first IN must run only after all three words have transferred.
    check_port 0x40,6
    check_port 0x41,0x10
    check_port 0x42,0
    check_port 0x44,6
    check_port 0x45,0x11
    check_port 0x46,0
    check_port 0x47,0
    check_port 0x48,0
    mov si,0x1000
    mov di,0x1100
    mov cx,3
    repe cmpsw
    jne failed
    mov word [0x402],1

    ; Reverse copy, keeping the byte order within each word unchanged.
    put_port 0x40,5
    put_port 0x41,0x10
    put_port 0x44,5
    put_port 0x45,0x12
    put_port 0x46,7
    put_port 0x48,0xff
    check_port 0x40,0xfe
    check_port 0x41,0x0f
    check_port 0x42,0
    check_port 0x44,0xfe
    check_port 0x45,0x11
    check_port 0x46,0
    check_port 0x47,0
    check_port 0x48,0x40
    mov si,0x1000
    mov di,0x1200
    mov cx,3
    repe cmpsw
    jne failed
    mov word [0x404],1
    mov word [0x400],0x600d
    mov bx,0x00f0                  ; green = both copies and readbacks passed
    jmp display

failed:
    mov word [0x400],0x0bad
    mov bx,0x0f00                  ; red = cartridge's own assertions failed
display:
    mov [0xfe02],bx
    mov di,0x2000                  ; 2bpp tile 0, constant palette index 1
    mov ax,0x00ff
    mov cx,8
    rep stosw
    put_port 7,0                   ; tilemap 0, zero-initialized tile indices
    put_port 0,1                   ; Screen 1
    put_port 0x14,1                ; LCD on
    hlt
