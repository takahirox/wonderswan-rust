; SPDX-License-Identifier: CC0-1.0
; Original absolute return probe for Issue #81.
; Independent budgets and result layout: docs/return-timing.md.
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
    mov word [0x2ff4],0xbeef
    mov word [0x2ff6],0xbeef
    mov word [0x2ff8],0xface
    mov ax,0xffff
    out 0xa4,ax
    put 0xa2,1
    mov cx,4096
    xor bx,bx
    xor bp,bp
    push word 0xf046
    popf
    call 0:0x2000
    mov [di],ax
    mov [di+2],bx
    mov [di+4],bp
    mov [di+6],cx
    add di,12
%endmacro
    sample c3_even
    sample c3_odd
    sample c2_even
    sample c2_odd
    sample cb_even
    sample cb_odd
    sample ca_even
    sample ca_odd
    sample cf_even
    sample cf_odd
    inc word [0x406]
    ret

%macro body 3                 ; name, opcode, loaded destination parity
%1:
    in ax,0xa8                ; 2000
    mov dx,ax                 ; 2002
%%repeat:                     ; 2004, LOOP leaves FLAGS unchanged
    lea bx,[bx+1]
%if %2 == 0xc2 || %2 == 0xca
    push word 0xface          ; discarded by imm16 adjustment
%endif
%if %2 == 0xcf
    push word 0xf046          ; complete normalized FLAGS
%endif
%if %2 == 0xcb || %2 == 0xca || %2 == 0xcf
    push es                   ; CS=0, ES=0 in copied body
%endif
    push si                   ; loaded continuation, actual frame pop
    db %2
%if %2 == 0xc2 || %2 == 0xca
    dw 2
%endif
    times (($-%1) & 1) db 0xf4
    times %3 db 0xf4           ; unreachable poison, sets destination parity
%1_target:
    lea bp,[bp+1]
    loop %%repeat
    in ax,0xa8                ; end observation before its clocks advance
    mov [0x908],sp
    mov sp,0x2ff0             ; capture FLAGS without overwriting popped frame
    pushf
    pop si
    mov sp,[0x908]
    mov [di+8],si             ; CPU-observed full restored FLAGS
    sub dx,ax
    mov ax,dx
    cmp si,0xf046
    jne %%bad
    cmp sp,0x2ffa
    jne %%bad
%if %2 == 0xc3
    cmp word [0x2ff8],0x2000+%1_target-%1
    jne %%bad
    cmp word [0x2ff6],0xbeef
    jne %%bad
    cmp word [0x2ff4],0xbeef
%elif %2 == 0xc2
    cmp word [0x2ff8],0xface
    jne %%bad
    cmp word [0x2ff6],0x2000+%1_target-%1
    jne %%bad
    cmp word [0x2ff4],0xbeef
%elif %2 == 0xcb
    cmp word [0x2ff8],0
    jne %%bad
    cmp word [0x2ff6],0x2000+%1_target-%1
    jne %%bad
    cmp word [0x2ff4],0xbeef
%elif %2 == 0xca
    cmp word [0x2ff8],0xface
    jne %%bad
    cmp word [0x2ff6],0
    jne %%bad
    cmp word [0x2ff4],0x2000+%1_target-%1
%else
    cmp word [0x2ff8],0xf046
    jne %%bad
    cmp word [0x2ff6],0
    jne %%bad
    cmp word [0x2ff4],0x2000+%1_target-%1
%endif
    jne %%bad
    mov word [di+10],0x51a7
    retf
%%bad:
    hlt
    jmp %%bad
%1_end:
%endmacro
    body c3_even,0xc3,0
    body c3_odd,0xc3,1
    body c2_even,0xc2,0
    body c2_odd,0xc2,1
    body cb_even,0xcb,0
    body cb_odd,0xcb,1
    body ca_even,0xca,0
    body ca_odd,0xca,1
    body cf_even,0xcf,0
    body cf_odd,0xcf,1
