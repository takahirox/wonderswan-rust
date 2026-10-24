; SPDX-License-Identifier: CC0-1.0
; Original absolute external IRQ budget probe for Issue #69.
; 1024 latched IRQs, with no acknowledgement until the final handler.
; All measured instructions and handlers run from internal RAM.
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
    mov cx,128
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
    mov word [es:0],0             ; invalidate retained success before any sample
    xor ax,ax
    mov es,ax
    mov si,body
    mov di,0x2000
    mov cx,body_end-body
    cs rep movsb
    mov word [0x9c],0x2000+handler-body
    mov word [0x9e],0
    put 0xb0,0x20
    put 0xb2,0x80
    in al,0xa0
    and ax,2
    mov [0x404],ax
    mov di,0x408
    call probe
    cmp word [0x404],0
    je complete
%macro mode_pass 1
    put 0x60,%1
    call probe
%endmacro
    mode_pass 0x80
    mode_pass 0
    mode_pass 0xc0
    mode_pass 0
    mode_pass 0xe0
    mode_pass 0
complete:
    mov word [0x400],0x600d
    jmp publish

probe:
    mov [0x600],sp
    mov sp,0x2800
    call sample
    mov sp,0x2801
    call sample
    mov sp,[0x600]
    inc word [0x406]
    ret
sample:
    ; A real HBlank timer event latches the request while IF is clear.
    mov ax,1
    out 0xa4,ax
    put 0xa2,1
.wait:
    in al,0xb4
    test al,0x80
    jz .wait
    mov ax,0xffff
    out 0xa4,ax
    mov cx,1024
    mov si,1024
    xor bx,bx
    call 0:0x2000
    mov [di],ax                   ; elapsed HBlank ticks
    mov [di+2],bx                 ; handler count
    mov [di+4],cx                 ; application loop completed
    mov [di+6],si                 ; final handler completed
    add di,8
    ret

body:
    in ax,0xa8
    mov dx,ax
    sti
.spin:
    loop .spin                   ; one protected application instruction per IRQ
    cli
    in ax,0xa8
    sub dx,ax
    mov ax,dx
    retf
handler:
    inc bx
    dec si
    jnz .return
    mov al,0x80
    out 0xb6,al                   ; only final handler acknowledges the latch
.return:
    iret
body_end:

publish:
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,128
    rep movsw
.done:
    hlt
    jmp .done
