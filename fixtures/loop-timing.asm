; SPDX-License-Identifier: CC0-1.0
; Original absolute LOOP/LOOPE/LOOPNE probe for Issue #71.
; Independent budgets and result layout: docs/loop-timing.md.
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
%macro sample 2                ; body, initial BP (ZF source)
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
    mov si,4096
    xor bx,bx
    mov bp,%2
    test bp,bp                 ; all measured taken-path instructions preserve ZF
    call 0:0x2000
    mov [di],ax
    mov [di+2],bx
    mov [di+4],cx
    add di,8
%endmacro
    sample loop_even,0
    sample loop_odd,0
    sample loope_even,0
    sample loope_odd,0
    sample loopne_even,1
    sample loopne_odd,1
    sample loop_fall,0
    sample loope_fall,1
    sample loopne_fall,0
    inc word [0x406]
    ret

%macro taken_body 3
%1:
    in ax,0xa8
    mov dx,ax
%if %3
    nop                       ; before target, but inside interval: +2 clocks
%endif
%%target:
    lea bx,[bx+1]             ; independent iteration count without changing ZF
    %2 %%target
    in ax,0xa8
    sub dx,ax
    mov ax,dx
    mov word [di+6],0x51a7
    retf
%1_end:
%endmacro
    taken_body loop_even,loop,0
    taken_body loop_odd,loop,1
    taken_body loope_even,loope,0
    taken_body loope_odd,loope,1
    taken_body loopne_even,loopne,0
    taken_body loopne_odd,loopne,1

%macro fall_body 3
%1:
    in ax,0xa8
    mov dx,ax
%%repeat:
    mov cx,%3
    lea bx,[bx+1]
    test bp,bp
    %2 %%bad
    dec si
    jnz %%repeat
    in ax,0xa8
    sub dx,ax
    mov ax,dx
    mov word [di+6],0x51a7
    retf
%%bad:
    mov word [di+6],0x0bad
    retf
%1_end:
%endmacro
    fall_body loop_fall,loop,1
    fall_body loope_fall,loope,2
    fall_body loopne_fall,loopne,2
