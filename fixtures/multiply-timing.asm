; SPDX-License-Identifier: CC0-1.0
; Original absolute multiplication probe for Issue #85.
; Independent budgets and result layout: docs/multiplication-timing.md.
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
    mov sp,0x3f00
    mov di,0x400
    mov cx,3584
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
    mov cx,3584
    rep movsw
.done:
    hlt
    jmp .done

probe:
%macro sample 2               ; form 0..5, source: register/even RAM/odd RAM
    push di
    mov si,body_%1_%2
    mov di,0x2800
    mov cx,body_%1_%2_end-body_%1_%2
    cs rep movsb
    pop di
%if %1 < 2
    mov word [0x3200+%2/2],2
%elif %1 < 4
    mov word [0x3200+%2/2],7
%else
    mov word [0x3200+%2/2],-3
%endif
    mov ax,0xffff
    out 0xa4,ax
    put 0xa2,1
    mov dx,0x5aa5
    mov bp,1024
    xor si,si
    call 0:0x2800
    add di,16
%endmacro
%assign form 0
%rep 6
%assign source 0
%rep 3
    sample form,source
%assign source source+1
%endrep
%assign form form+1
%endrep
    inc word [0x406]
    ret

%macro body 2
body_%1_%2:
    in ax,0xa8                ; 2800: start snapshot before advance
    mov [0x3000],ax           ; 2802: preserve timer independently of DX product
    nop                       ; 2805: even repeat target
%%repeat:                     ; 2806
%if %1 < 2
    mov ax,3
%else
    mov ax,-3
%endif
%if %2 == 0
%if %1 < 2
    mov bx,2
%elif %1 < 4
    mov bx,7
%else
    mov bx,-3
%endif
%else
    mov bx,0x3200+%2/2
%endif
    push word 0xf002          ; 280C: odd immediate-word fetch
    popf                      ; 280F
%if %1 < 4
    db 0xf6 | (%1 & 1)
    db (0xe3 | ((%1/2) << 3))*(%2 == 0) + (0x27 | ((%1/2) << 3))*(%2 != 0)
%elif %1 == 4
    db 0x69
    db 0xc3*(%2 == 0) + 0x07*(%2 != 0)
    dw -257
%else
    db 0x6b
    db 0xc3*(%2 == 0) + 0x07*(%2 != 0)
    db -128
%endif
    pushf
    pop cx                    ; capture full FLAGS before DEC
    lea si,[si+1]             ; exact execution count without altering FLAGS
    dec bp
    jnz %%repeat
    mov [di+2],ax
    mov [di+4],dx
    in ax,0xa8                ; terminal snapshot before its clocks advance
    mov dx,[0x3000]
    sub dx,ax
    mov [di],dx
    mov [di+6],cx
    mov [di+8],si
    mov [di+10],bp
    mov [di+12],bx
    cmp sp,0x3efa
    jne %%bad
    mov word [di+14],0x51a7
    retf
%%bad:
    hlt
    jmp %%bad
body_%1_%2_end:
%endmacro
%assign form 0
%rep 6
%assign source 0
%rep 3
    body form,source
%assign source source+1
%endrep
%assign form form+1
%endrep
