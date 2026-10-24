; SPDX-License-Identifier: CC0-1.0
; Original absolute direct CALL probe for Issue #77.
; Independent budgets and result layout: docs/call-timing.md.
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
    sample near_even
    sample near_odd
    sample far_even
    sample far_odd
    inc word [0x406]
    ret

%macro body 3               ; label, far, destination parity
%1:
    in ax,0xa8              ; RAM 2000: interval starts at observation
    mov dx,ax              ; RAM 2002
%%repeat:                  ; RAM 2004
    lea bx,[bx+1]          ; RAM 2004, execution count
%if %2 == 0
    call near %%target     ; RAM 2007, saves 200A
%else
    call 0:(0x2000+%%target-%1) ; RAM 2007, saves CS=0 then IP=200C
%endif
    dec cx
    jnz %%repeat           ; even destination; final branch falls through
    in ax,0xa8             ; terminal observation excludes IN clocks
    sub dx,ax
    mov ax,dx
    cmp sp,0x2ffa
    jne %%bad
%if %2 == 0
    cmp word [0x2ff8],0x200a
    jne %%bad
%else
    cmp word [0x2ff6],0x200c
    jne %%bad
    cmp word [0x2ff8],0
    jne %%bad
%endif
    mov word [di+8],0x51a7 ; continuation and stack integrity passed
    retf
%%bad:
    hlt
    jmp %%bad
    times (($-%1) & 1) db 0xf4
    times %3 db 0xf4
%%target:                  ; explicitly even/odd RAM destination
    lea bp,[bp+1]          ; independent destination count
%if %2 == 0
    ret
%else
    retf
%endif
%1_end:
%endmacro
    body near_even,0,0
    body near_odd,0,1
    body far_even,1,0
    body far_odd,1,1
