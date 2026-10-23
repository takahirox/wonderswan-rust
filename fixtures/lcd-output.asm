; SPDX-License-Identifier: CC0-1.0
; Original Issue #37 line-polling/output-stage probe. No external assets/code.
bits 16
cpu 186
org 0
%macro put 2
    mov al,%2
    out %1,al
%endmacro
%macro wait_line 1
%%poll:
    in al,2
    cmp al,%1
    jne %%poll
%endmacro
start:
    cli
    cld
    xor ax,ax
    mov ds,ax
    mov ss,ax
    mov sp,0x3000
    mov es,ax
    mov di,0x800
    mov cx,1024
    rep stosw                    ; Screen 1: tile 0, palette 0
    mov di,0x1000
    mov ax,1 | (4 << 9)
    mov cx,1024
    rep stosw                    ; Screen 2: transparent palette 4, tile 1
    mov di,0x2000
    mov ax,0x00ff
    mov cx,8
    rep stosw                    ; tile 0: index 1
    mov ax,0xf000
    mov cx,8
    rep stosw                    ; tile 1: index 2 left four, transparent right four
    mov ax,0xffff
    mov cx,8
    rep stosw                    ; tile 2: index 3
    mov word [0x1800],2 | (4 << 9) | 0x2000
    mov word [0x1802],0x1028      ; priority sprite, x16 y40
    put 4,12
    put 5,0
    put 6,1
    put 7,0x21
    put 0x20,0x10                ; palette 0: index 1 -> shade 1
    put 0x21,0x30                ; index 3 -> shade 3
    put 0x29,0x02                ; Screen 2: index 2 -> shade 2
    put 0x39,0x30                ; sprite: index 3 -> shade 3
    in al,0xa0
    and ax,2
    mov bp,ax
    test bp,bp
    jz .mono
    put 0x60,0x80                ; color 2bpp planar; same identities as mono
.mono:
    mov ax,0x1000
    mov es,ax
    mov ax,[es:0]
    cmp ax,0xffff
    jne .count
    xor ax,ax
.count:
    inc ax
    mov [es:0],ax                ; retained fresh CPU boot marker
    mov [0x400],ax
    mov word [es:2],0
    mov word [0x402],0
    put 0,7
    put 0x14,1
.frame:
    wait_line 150               ; warmup guarantees first OAM latch before probe
    test bp,bp
    jz .base_mono
    mov word [0xfe02],0x0f00     ; base red, green, blue
    mov word [0xfe84],0x00f0
    mov word [0xfe06],0x000f
    mov word [0xff86],0x000f
    jmp .base_done
.base_mono:
    put 0x1c,0
    put 0x1d,0xc0               ; base white, white, gray 51
.base_done:
    mov word [0x200e],0x00ff     ; restore tile row 7 for next frame
    wait_line 0
    wait_line 40
    test bp,bp
    jz .change_mono
    mov word [0xfe02],0x0ff0     ; row 39's index 1 -> yellow
    mov word [0xfe84],0x0f0f     ; foreground -> magenta
    mov word [0xff86],0x00ff     ; sprite -> cyan
    jmp .changed
.change_mono:
    put 0x1c,0xf0               ; shade 1 -> black
    put 0x1d,0xa5               ; shade 2 -> gray 170, shade 3 -> gray 85
.changed:
    mov word [0x200e],0xffff    ; pending row39 must retain index1, not refetch3
    in al,2
    xor ah,ah
    mov [0x404],ax
    mov [es:4],ax               ; independent literal 40 verifies bounded writes
    wait_line 144
    test bp,bp
    jz .last_mono
    mov word [0xfe06],0x0a53     ; pending row143 index3 -> (170,85,51)
    mov word [0xfe84],0x0456     ; foreground -> (68,85,102)
    jmp .last_done
.last_mono:
    put 0x1d,0x86               ; shade2 -> gray153, shade3 -> gray119
.last_done:
    in al,2
    xor ah,ah
    mov [0x406],ax
    mov [es:6],ax               ; independent literal 144
    inc word [0x402]
    mov ax,[0x402]
    mov [es:2],ax               ; fresh completed probe passes
    mov word [0x408],0x600d
    mov word [es:8],0x600d
    jmp .frame
