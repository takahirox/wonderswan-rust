; SPDX-License-Identifier: CC0-1.0
; Original Issue #43 multiplication probe. No imported code, assets or firmware.
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
    mov cx,100
    rep stosw
    mov ax,0x1000
    mov es,ax
    mov ax,[es:2]                ; retained count only, never saved completion
    cmp ax,0xffff
    jne .count
    xor ax,ax
.count:
    inc ax
    mov [0x402],ax
    mov word [es:0],0            ; invalidate saved completion before testing
    xor ax,ax
    mov es,ax
    in al,0xa0
    and ax,2
    mov [0x404],ax               ; physical model bit, independent of port 60
    mov word [0x800],0xf002      ; unsigned expected flags without overflow
    test ax,ax
    jz .mono
    mov word [0x800],0xf042
.mono:
    mov ax,[0x800]
    mov word [0x802],0
    mov word [0x806],6
    cmp ax,0xf002
    je .expect
    mov word [0x802],1
    mov word [0x806],7
.expect:
    or ax,0x0c01
    mov [0x804],ax
    mov di,0x408
    call probe                  ; all models begin in mono graphics
    inc word [0x406]
    cmp word [0x404],0
    je complete
    put 0x60,0x80
    call probe                  ; physical Color/Crystal, color graphics
    inc word [0x406]
    put 0x60,0
    call probe                  ; physical Color/Crystal, back to mono graphics
    inc word [0x406]
complete:
    cmp sp,0x3000
    jne failed
    mov word [0x400],0x600d
    jmp display

; Capture actual product/high/FLAGS plus actual subsequent branch decisions.
; None of the MOV/PUSH/POP/Jcc instructions before branch collection change flags.
%macro observe 5
    pushf
    pop si
    mov [di],%1
    mov [di+2],dx
    mov [di+4],si
    mov bp,0
    jnz %%nz
    or bp,1
%%nz:
    ; OR changed flags: restore the actual saved multiplication flags.
    push si
    popf
    jnc %%nc
    or bp,2
%%nc:
    push si
    popf
    jno %%no
    or bp,4
%%no:
    mov [di+6],bp
    add di,8
    cmp %1,%2
    jne failed
    cmp dx,%3
    jne failed
    cmp si,%4
    jne failed
    cmp bp,%5
    jne failed
%endmacro

probe:
    mov dx,0x1357
    mov ax,3
    mov bx,2
    push word 0xf002
    popf
    mul bl
    ; model-specific comparison cannot run until after branch capture
    observe ax,6,0x1357,[0x800],[0x802]
    mov ax,0xffff
    mov bx,2
    push word 0xfcd7
    popf
    mul bx
    ; overflow FLAGS include preserved DF and model-dependent ZF
    observe ax,0xfffe,1,[0x804],[0x806]

    mov dx,0x1357
    mov ax,0xfffd
    mov bx,7
    push word 0xf002
    popf
    imul bl
    observe ax,0xffeb,0x1357,0xf042,1

    mov ax,0x8000
    mov bx,0xffff
    push word 0xfcd7
    popf
    imul bx
    observe ax,0x8000,0,0xfc43,7

    mov ax,0x2468
    mov dx,0x1357
    mov bx,0xfffd
    push word 0xf002
    popf
    db 0x69,0xcb,7,0             ; IMUL CX,BX,imm16
    observe cx,0xffeb,0x1357,0xf042,1
    cmp ax,0x2468
    jne failed
    cmp bx,0xfffd
    jne failed

    mov bx,0x100
    push word 0xfcd7
    popf
    db 0x6b,0xdb,0x80            ; IMUL BX,BX,-128 (source/destination alias)
    observe bx,0x8000,0x1357,0xf442,1
    cmp ax,0x2468
    jne failed

    mov word [0x900],0x100
    push word 0xfcd7
    popf
    db 0x26,0x69,0x0e,0,9,0,1   ; IMUL CX,[ES:0900],0100
    observe cx,0,0x1357,0xfc43,7

    mov word [0x901],0x101
    push word 0xf002
    popf
    db 0x26,0x6b,0x0e,1,9,0x80  ; IMUL CX,[ES:0901],-128
    observe cx,0x7f80,0x1357,0xf843,7
    cmp ax,0x2468
    jne failed
    ret

failed:
    mov word [0x400],0x0bad
display:
    cli
    cld
    mov sp,0x3000
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,100
    rep movsw
    xor ax,ax
    mov es,ax
    in al,0xa0
    test al,2
    jz .mono
    put 0x60,0x80
    mov bx,0x00f0
    cmp word [0x400],0x600d
    je .color
    mov bx,0x0f00
.color:
    mov [0xfe02],bx
    jmp .tile
.mono:
    mov ax,0x7654
    cmp word [0x400],0x600d
    je .gray
    mov ax,0x7653
.gray:
    out 0x20,ax
    put 0x1c,0x10
    put 0x1d,0x32
    put 0x1e,0x54
    put 0x1f,0x76
.tile:
    mov di,0x2000
    mov ax,0x00ff
    mov cx,8
    rep stosw
    put 7,0
    put 0,1
    put 0x14,1
    hlt
