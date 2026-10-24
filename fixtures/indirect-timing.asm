; SPDX-License-Identifier: CC0-1.0
; Original absolute indirect CALL/JMP probe for Issue #79.
; Independent budgets and result layout: docs/indirect-control-timing.md.
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
    mov cx,512
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
    mov cx,512
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
    mov si,0x2000+%1_target-%1
    mov [0x900],si
    mov word [0x902],0
    mov word [0x2ff6],0xbeef
    mov word [0x2ff8],0xface
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
    sample call_reg_even
    sample call_reg_odd
    sample call_mem_even
    sample call_mem_odd
    sample call_far_even
    sample call_far_odd
    sample jmp_reg_even
    sample jmp_reg_odd
    sample jmp_mem_even
    sample jmp_mem_odd
    sample jmp_far_even
    sample jmp_far_odd
    inc word [0x406]
    ret

%macro body 3              ; name, form 0..5, destination parity
%1:
    in ax,0xa8             ; 2000, start observation
    mov dx,ax              ; 2002
%%repeat:                  ; 2004
    lea bx,[bx+1]
%if %2 == 0
    call si                ; 2007, saves 2009
%elif %2 == 1
    call word [0x900]      ; 2007, saves 200B
%elif %2 == 2
    call far [0x900]       ; 2007, saves 200B and CS=0
%elif %2 == 3
    jmp si
%elif %2 == 4
    jmp word [0x900]
%else
    jmp far [0x900]
%endif
%if %2 >= 3
    times (($-%1) & 1) db 0xf4
    times %3 db 0xf4
%1_target:
    lea bp,[bp+1]
%endif
    dec cx
    jnz %%repeat
    in ax,0xa8             ; end observation before IN clocks advance
    sub dx,ax
    mov ax,dx
    cmp sp,0x2ffa
    jne %%bad
%if %2 == 0
    cmp word [0x2ff8],0x2009
%elif %2 == 1
    cmp word [0x2ff8],0x200b
%elif %2 == 2
    cmp word [0x2ff8],0
%else
    cmp word [0x2ff8],0xface
%endif
    jne %%bad
%if %2 == 2
    cmp word [0x2ff6],0x200b
%else
    cmp word [0x2ff6],0xbeef
%endif
    jne %%bad
    mov word [di+8],0x51a7
    retf
%%bad:
    hlt
    jmp %%bad
%if %2 < 3
    times (($-%1) & 1) db 0xf4
    times %3 db 0xf4
%1_target:
    lea bp,[bp+1]
%if %2 == 2
    retf
%else
    ret
%endif
%endif
%1_end:
%endmacro
    body call_reg_even,0,0
    body call_reg_odd,0,1
    body call_mem_even,1,0
    body call_mem_odd,1,1
    body call_far_even,2,0
    body call_far_odd,2,1
    body jmp_reg_even,3,0
    body jmp_reg_odd,3,1
    body jmp_mem_even,4,0
    body jmp_mem_odd,4,1
    body jmp_far_even,5,0
    body jmp_far_odd,5,1
