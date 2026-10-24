; SPDX-License-Identifier: CC0-1.0
; Original absolute shift/rotate probe for Issue #83.
; Independent budgets and result layout: docs/shift-rotate-timing.md.
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
%macro sample 3               ; operation, width, profile
    push di
    mov si,body_%1_%2_%3
    mov di,0x2800
    mov cx,body_%1_%2_%3_end-body_%1_%2_%3
    cs rep movsb
    pop di
    mov ax,0xffff
    out 0xa4,ax
    put 0xa2,1
%if %3 == 0
    mov cx,1
%elif %3 == 1
    mov cx,0
%elif %3 == 2
    mov cx,31
%elif %3 == 3
    mov cx,32
%else
    mov cx,255
%endif
    mov bp,1024
    xor bx,bx
    call 0:0x2800
    add di,14
%endmacro
%assign op 0
%rep 8
%if op != 6
%assign wide 0
%rep 2
%assign profile 0
%rep 5
    sample op,wide,profile
%assign profile profile+1
%endrep
%assign wide wide+1
%endrep
%endif
%assign op op+1
%endrep
    inc word [0x406]
    ret

%macro body 3
body_%1_%2_%3:
    in ax,0xa8                ; 2800: start observation
    mov dx,ax                 ; 2802
%%repeat:                     ; 2804: even target
%if %3 == 1 || %3 == 3
    mov ax,0x4000*(%2)+0xa540*(1-%2)
%elif %3 == 4
    mov ax,0xffff*(%2)+0xa5ff*(1-%2)
%else
    mov ax,0x8001*(%2)+0xa581*(1-%2)
%endif
    push word 0xf002          ; 2807: aligned immediate, aligned stack
    popf                      ; 280A
%if %3 == 0
    db 0xd0 | %2,0xc0 | (%1 << 3)
%elif %3 <= 2
    db 0xd2 | %2,0xc0 | (%1 << 3)
%else
    db 0xc0 | %2,0xc0 | (%1 << 3)
%if %3 == 3
    db 32
%else
    db 255
%endif
%endif
    pushf
    pop si                    ; capture full shift FLAGS before counter DEC
    lea bx,[bx+1]
    dec bp
    jnz %%repeat
    mov [di+2],ax             ; result before timer IN clobbers AX
    in ax,0xa8                ; terminal observation before its clocks advance
    sub dx,ax
    mov [di],dx
    mov [di+4],si
    mov [di+6],bx
    mov [di+8],bp
    mov [di+10],cx
    cmp sp,0x3efa
    jne %%bad
    mov word [di+12],0x51a7
    retf
%%bad:
    hlt
    jmp %%bad
body_%1_%2_%3_end:
%endmacro
%assign op 0
%rep 8
%if op != 6
%assign wide 0
%rep 2
%assign profile 0
%rep 5
    body op,wide,profile
%assign profile profile+1
%endrep
%assign wide wide+1
%endrep
%endif
%assign op op+1
%endrep
