; SPDX-License-Identifier: CC0-1.0
; Original absolute Jcc/JCXZ probe for Issue #75.
; Independent budgets and result layout: docs/jcc-timing.md.
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
%macro sample 2
    push di
    mov si,%1
    mov di,0x2000
    mov cx,%1_end-%1
    cs rep movsb
    pop di
    mov ax,0xffff
    out 0xa4,ax
    put 0xa2,1
    mov si,4096
    mov cx,%2
    xor bx,bx
    xor bp,bp
    call 0:0x2000
    mov [di],ax
    mov [di+2],bx
    mov [di+4],bp
    mov [di+6],cx
    add di,10
%endmacro
    sample jz_even,0
    sample jz_odd,0
    sample jz_fall,1
    sample jcxz_even,0
    sample jcxz_odd,0
    sample jcxz_fall,1
    inc word [0x406]
    ret

%macro body 3               ; label, opcode, path (0 even / 1 odd / 2 fall)
%1:
    in ax,0xa8              ; RAM 2000: interval begins at observation
    mov dx,ax              ; RAM 2002
%%repeat:                  ; RAM 2004: even harness JNZ target
    lea bx,[bx+1]          ; independent execution count (4 clocks)
    cmp cx,0               ; RAM 2007: identical flag setup for both forms
%if %3 == 2
    db %2,3                ; RAM 200A: unused target would skip BP increment
%else
    db %2,4+%3             ; RAM 200A -> RAM 2010/2011
    times 4+%3 db 0xf4     ; poison HLTs must be skipped
%endif
    lea bp,[bp+1]          ; independent destination/fallthrough count
    dec si
    jnz %%repeat
    in ax,0xa8             ; terminal observation excludes this IN's clocks
    sub dx,ax
    mov ax,dx
    mov word [di+8],0x51a7 ; reached after exactly 4096 iterations
    retf
%1_end:
%endmacro
    body jz_even,0x74,0
    body jz_odd,0x74,1
    body jz_fall,0x74,2
    body jcxz_even,0xe3,0
    body jcxz_odd,0xe3,1
    body jcxz_fall,0xe3,2
