; SPDX-License-Identifier: CC0-1.0
; Original absolute direct JMP probe for Issue #73.
; Independent budgets and result layout: docs/jmp-timing.md.
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
    mov cx,256
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
    mov word [es:0],0          ; invalidate retained completion first
    xor ax,ax
    mov es,ax
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
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,256
    rep movsw
.done:
    hlt
    jmp .done

probe:
%macro sample 1
    push di
    mov si,%1
    mov di,0x2000
    mov cx,%1_end-%1
    cs rep movsb
    pop di
    mov ax,0xffff
    out 0xa4,ax
    put 0xa2,1
    mov cx,4096
    xor bx,bx
    xor bp,bp
    call 0:0x2000
    mov [di],ax
    mov [di+2],bx
    mov [di+4],bp
    mov [di+6],cx
    add di,10
%endmacro
    sample short_even
    sample short_odd
    sample near_even
    sample near_odd
    sample far_even
    sample far_odd
    inc word [0x406]
    ret

%macro body 3               ; label, form, destination parity
%1:
    in ax,0xa8              ; RAM 2000: interval starts at the observation
    mov dx,ax              ; RAM 2002
%%repeat:                  ; RAM 2004
    lea bx,[bx+1]          ; independent execution count
%if %2 == 0
    jmp short %%target     ; RAM 2007, two bytes
    times 3+%3 db 0xf4     ; poison skipped by the jump
%elif %2 == 1
    jmp near %%target      ; RAM 2007, three bytes
    times 2+%3 db 0xf4
%else
    jmp 0:(0x2000+%%target-%1) ; RAM 2007, five bytes, absolute RAM offset
    times 2+%3 db 0xf4
%endif
%%target:                  ; EB/E9 200C/200D, EA 200E/200F
    lea bp,[bp+1]          ; independent destination count
    dec cx
    jnz %%repeat
    in ax,0xa8             ; terminal observation excludes this IN's clocks
    sub dx,ax
    mov ax,dx
    mov word [di+8],0x51a7 ; continuation reached after CX expired
    retf
%1_end:
%endmacro
    body short_even,0,0
    body short_odd,0,1
    body near_even,1,0
    body near_odd,1,1
    body far_even,2,0
    body far_odd,2,1
