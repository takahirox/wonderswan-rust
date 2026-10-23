; SPDX-License-Identifier: CC0-1.0
; Original Issue #23 probe. No third-party test code, assets or firmware.
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
    ; Run all width edits from RAM. Never assert mismatched-width ROM data.
    mov si,body
    mov di,0x800
    mov cx,body_end-body
    cs rep movsb
    jmp 0:0x800
body:
; Labels in the copied code resolve to its RAM destination.
%define R(label) (0x800 + label - body)
    in al,0xa0
    mov [0x406],al                 ; header-seeded fast/16 handoff
    and ax,0x0083
    mov bp,ax                      ; immutable model/status/lock
    or al,4
    cmp al,[0x406]
    jne failed
    mov si,R(controls)
    mov cx,controls_end-controls
.controls:
    lodsb
    mov bl,al
    out 0xa0,al
    in al,0xa0
    and bl,12
    mov dx,bp
    or bl,dl
    cmp al,bl
    jne failed
    loop .controls
    put 0xa0,4
    put 0xc2,0
    mov ax,0x2000
    mov es,ax
    mov ax,[es:0]                  ; correctly configured physical ROM
    cmp ax,0x3131
    jne failed
    mov [0x408],ax
    put 0xa0,12                    ; extra wait, width still matches
    cmp word [es:1],0x3131         ; odd data access
    jne failed
    mov ax,bp
    test al,2
    jnz color
    mov word [0x400],0x600d
    mov bx,0x7654                  ; mono pass shade 170 at palette index 1
    jmp display
color:
    put 0x60,0x80
    put 0xa0,4
    put 0x40,0
    put 0x41,0
    put 0x42,2
    put 0x44,0
    put 0x45,0x10
    put 0x46,2
    put 0x47,0
    put 0x48,0x80
    check 0x46,0
    cmp word [0x1000],0x3131
    jne failed
    ; Reject both slow/16 and fast/byte ROM without reading mismatched data.
    put 0xa0,12
    put 0x46,2
    put 0x48,0x80
    check 0x46,2
    check 0x48,0
    put 0xa0,0
    put 0x48,0x80
    check 0x46,2
    check 0x48,0
    cmp word [0x1002],0
    jne failed
    mov word [0x402],1
    ; SDMA: change wait after the first sample while the engine stays active.
    ; Rate 0 leaves six event ticks between samples, enough for CPU polling.
    put 0xa0,4
    put 0x4a,0
    put 0x4b,0
    put 0x4c,2
    put 0x4e,2
    put 0x4f,0
    put 0x50,0
    put 0x52,0x80
.wait_first:
    in al,0x4e
    cmp al,1
    jne .wait_first
    put 0xa0,12
.wait_done:
    in al,0x52
    test al,0x80
    jnz .wait_done
    check 0x89,0x31
    check 0x4a,2
    check 0x4e,0
    ; Hyper Voice repeats the test with wait going from slow to fast.
    put 0x6b,0x70                  ; reset, route both inputs
    put 0x6a,0x89                  ; signed shift 1
    put 0x4a,0
    put 0x4e,2
    put 0x52,0x90
.hyper_first:
    in al,0x4e
    cmp al,1
    jne .hyper_first
    put 0xa0,4
.hyper_done:
    in al,0x52
    test al,0x80
    jnz .hyper_done
    check 0x68,0x31
    check 0x69,0x31
    check 0x4a,2
    check 0x4e,0
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
    mov bx,0x7653                  ; mono failure shade 187
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
controls:
    db 0,255,255,0,0x55,0xaa,0x55,0xaa,4,8,12,0x70
controls_end:
body_end:
