; SPDX-License-Identifier: CC0-1.0
; Original Issue #63 CPU IN/OUT probe; no external code or firmware.
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
    mov ss,ax
    mov sp,0x3000
    mov ax,0x1000
    mov es,ax
    mov word [es:0],0          ; invalidate any old pass before executing
    mov ax,[es:0x100]          ; retained execution counter, independent of results
    cmp ax,0xffff
    jne .counter
    xor ax,ax
.counter:
    inc ax
    mov [es:0x100],ax
    mov di,2
    mov bl,0x13
    mov cx,1
    call read
    mov bl,0x15
    mov cx,7
    call read                  ; initial retained state, before any writes
    in al,0xa0
    test al,2
    jz .mono
    mov bp,7
    jmp .modes
.mono:
    mov bp,1
.modes:
    xor dx,dx
.loop:
    in al,0xa0
    test al,2
    jz .mode
    mov al,dl
    out 0x60,al
.mode:
    mov al,dl
    stosb
    push dx
    mov bl,0x10
    mov si,reset
    mov cx,1
    call write
    mov bl,0x12
    mov si,status
    mov cx,1
    call write
    mov bl,0x13
    mov cx,1
    call read
    mov bl,0x14
    mov si,calendar
    mov cx,7
    call write
    mov bl,0x15
    mov cx,7
    call read
    mov bl,0x16
    mov si,time
    mov cx,3
    call write
    mov bl,0x17
    mov cx,3
    call read
    pop dx
    ; Explicit 00/80/00/C0/00/E0/00 sequence.
    inc dh
    mov si,modes
    xor ax,ax
    mov al,dh
    add si,ax
    mov dl,[cs:si]
    dec bp
    jnz .loop
    mov word [es:0],0x630d
    hlt

; Preserve the mapper's preload and terminal READY behavior.
write:
    push bp
    mov bp,4096
    cs lodsb
    out 0xcb,al
    mov al,bl
    out 0xca,al
    dec cx
.wait:
    in al,0xca
    test al,0x80
    jnz .ready
    dec bp
    jnz .wait
    jmp timeout
.ready:
    test al,0x10
    jz .done
    cs lodsb
    out 0xcb,al
    dec cx
    jmp .wait
.done:
    xor al,al
    out 0xcb,al
    pop bp
    ret
read:
    push bp
    mov bp,4096
    mov al,bl
    out 0xca,al
.wait:
    in al,0xca
    test al,0x80
    jnz .ready
    dec bp
    jnz .wait
    jmp timeout
.ready:
    in al,0xcb
    stosb
    loop .wait
    pop bp
    ret
timeout:
    mov word [es:0],0x0bad
    hlt
reset: db 0xa5
status: db 0x7f
calendar: db 0x24,0x02,0x28,0x03,0x23,0x59,0x58
time: db 0x12,0x34,0x56
modes: db 0,0x80,0,0xc0,0,0xe0,0
