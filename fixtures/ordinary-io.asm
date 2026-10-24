; SPDX-License-Identifier: CC0-1.0
; Original absolute ordinary IN/OUT timing observations. No third-party data.
; 2048 transfers in RAM; DX forms have one explicit two-clock NOP per transfer.
; Interval includes the initial IN AX,A8 (8) and MOV BP,AX (3), before the
; terminal IN reads the counter. See docs/ordinary-io-timing.md for derivation.
bits 16
cpu 186
org 0
start:
    cli
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,0x1800
    cld
    mov di,0x400
    mov cx,128
    rep stosw                    ; clear volatile results on every fresh boot
    mov ax,0x1000
    mov es,ax
    mov ax,[es:0]
    cmp ax,0xffff
    jne .retained
    xor ax,ax
.retained:
    inc ax
    mov [0x400],ax
    mov [es:0],ax
    mov word [es:2],0             ; invalidate retained success before measuring
    xor ax,ax
    mov es,ax
    push ds
    push cs
    pop ds
    mov si,body
    mov di,0x2000
    mov cx,body_end-body
    rep movsb
    pop ds
    mov ax,0xffff
    out 0xa4,ax
    mov al,1
    out 0xa2,al                   ; HBlank timer, CLI: no interrupt service
    mov word [0x404],1
    in al,0xa0
    test al,2
    jz .mono
    mov word [0x404],7
.mono:
    mov word [0x500],0            ; phase index, separate from published results
    mov bx,0x406
.phase:
    mov si,[0x500]
    mov al,[cs:modes+si]
    xor ah,ah
    mov [bx],ax
    add bx,2
    cmp word [0x404],1
    je .mode_ready
    out 0x60,al
.mode_ready:
%macro sample 3                   ; opcode bytes, DX port, lower tick bound
    push bx
    mov di,0x2000+transfer-body
    mov ax,%1
    mov cx,2048
    rep stosw                    ; patch every transfer before executing burst
    mov dx,%2
    call 0:0x2000
    pop bx
    mov [bx],ax
    add bx,2
    cmp ax,%3
    jb failed
    cmp ax,%3+1
    ja failed
%endmacro
    sample 0xbce4,0,64            ; immediate byte IN: (11+2048*8)/256
    sample 0xbce5,0,64            ; immediate eligible even word IN
    sample 0xbbe5,0,72            ; immediate split odd word IN
    sample 0xbce6,0,64            ; immediate byte OUT
    sample 0xbce7,0,64            ; immediate eligible even word OUT
    sample 0xbbe7,0,72            ; immediate split odd word OUT
    sample 0x90ec,0xbc,72         ; DX byte IN + NOP: (11+2048*(7+2))/256
    sample 0x90ed,0xbc,72         ; DX eligible even word IN + NOP
    sample 0x90ed,0xbb,80         ; DX split odd word IN + NOP
    sample 0x90ee,0xbc,72         ; DX byte OUT + NOP
    sample 0x90ef,0xbc,72         ; DX eligible even word OUT + NOP
    sample 0x90ef,0xbb,80         ; DX split odd word OUT + NOP
    inc word [0x500]
    mov ax,[0x500]
    cmp ax,[0x404]
    jb .phase
    mov word [0x402],0x600d
    jmp publish
failed:
    mov word [0x402],0x0bad
publish:
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,128
    rep movsw                    ; mirror CPU-written observations to local SRAM
.done:
    hlt
    jmp .done
modes: db 0,0x80,0,0xc0,0,0xe0,0
body:
    in ax,0xa8
    mov bp,ax
transfer:
    times 2048 dw 0xbce4
    in ax,0xa8
    sub bp,ax
    mov ax,bp
    retf
body_end:
