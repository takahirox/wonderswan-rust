; SPDX-License-Identifier: CC0-1.0
; Original Issue #67 trap probe. No imported code, assets or firmware.
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
    mov cx,900
    rep stosw
    mov ax,0x1000
    mov es,ax
    mov ax,[es:2]
    cmp ax,0xffff
    jne .count
    xor ax,ax
.count:
    inc ax
    mov [0x402],ax
    mov word [es:0],0           ; invalidate old saved completion
    xor ax,ax
    mov es,ax
    mov word [4],handler
    mov word [6],0xf000
    mov word [0x1800],0x5aa5
    mov word [0x1a00],0         ; memory SS source
    in al,0xa0
    and ax,2
    mov [0x404],ax
    mov bp,0x408
    call probe
    inc word [0x406]
    cmp word [0x404],0
    je complete
%macro mode_pass 1
    put 0x60,%1
    call probe
    inc word [0x406]
%endmacro
    mode_pass 0x80
    mode_pass 0
    mode_pass 0xc0
    mode_pass 0
    mode_pass 0xe0
    mode_pass 0
complete:
    cmp sp,0x3000
    jne failed
    mov word [0x400],0x600d
    jmp display

; Each case records actual count, continuation BX and nine words per frame.
; The handler preserves the application registers and clears stacked TF only
; after the required count. All payload addresses are fixed RAM literals.
%macro run_case 3
    mov si,%1
    mov di,0x1000
    mov cx,%1_end-%1
    cs rep movsb
    mov word [0x1a04],0
    mov word [0x1a06],%3
    mov [0x1a08],bp
    add word [0x1a08],4
    mov cx,%2
    mov si,0x1800
    mov di,0x1900
    mov bx,0
    xor ax,ax
    call 0:0x1000
    mov ax,[0x1a04]
    mov [bp],ax
    mov [bp+2],bx
    cmp ax,%3
    jne failed
    cmp bx,0x1234
    jne failed
    mov bp,[0x1a08]
%endmacro
probe:
    run_case nop_case,2,1
    run_case sti_case,2,1
    run_case rep_case,2,2
    run_case rep_case,0,1
    run_case halt_case,2,1
    run_case ss_case,2,1
    run_case twice_case,2,2
    run_case memory_ss_case,2,1
    run_case pop_ss_case,2,1
    run_case ds_case,2,1
    ret

handler:
    push bp
    mov bp,sp
    push ax
    push di
    pushf
    pop ax
    mov di,[0x1a08]
    mov [di+6],ax              ; live handler FLAGS, before arithmetic
    mov ax,[bp+2]
    mov [di],ax                ; saved IP
    mov ax,[bp+4]
    mov [di+2],ax              ; saved CS
    mov ax,[bp+6]
    mov [di+4],ax              ; saved full FLAGS
    lea ax,[bp+2]
    mov [di+8],ax              ; entry SP
    mov [di+10],cx
    mov [di+12],si
    mov ax,[bp-4]
    mov [di+14],ax             ; original DI
    mov [di+16],bx
    add di,18
    mov [0x1a08],di
    inc word [0x1a04]
    mov ax,[0x1a04]
    cmp ax,[0x1a06]
    jb .return
    and word [bp+6],0xfeff     ; stop after the requested application boundary
.return:
    pop di
    pop ax
    pop bp
    iret

; PUSH FLAGS; POPF activates TF, then the selected instruction boundary.
%macro payload 2+
%1:
    push word 0xf102
    popf
    db %2
    mov bx,0x1234
    retf
%1_end:
%endmacro
payload nop_case,0x90
payload sti_case,0xfb
payload rep_case,0xf0,0x26,0xf3,0xa4
payload halt_case,0xf4
payload ss_case,0x8e,0xd0,0x90
payload twice_case,0x90,0x90
payload memory_ss_case,0x8e,0x16,0x00,0x1a,0x90
pop_ss_case:
    push word 0
    push word 0xf102
    popf
    pop ss
    nop
    mov bx,0x1234
    retf
pop_ss_case_end:
payload ds_case,0x8e,0xd8,0x90

failed:
    mov word [0x400],0x0bad
display:
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,900
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
