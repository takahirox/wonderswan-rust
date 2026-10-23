; SPDX-License-Identifier: CC0-1.0
; Original Issue #51 PUSH SP probe. No imported code, assets or firmware.
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
    mov cx,571
    rep stosw                  ; clear every volatile observation on every boot
    mov word [0x900],0         ; cumulative failure, never read from saved results
    mov ax,0x1000
    mov es,ax
    mov ax,[es:2]              ; retain only the execution counter
    cmp ax,0xffff
    jne .count
    xor ax,ax
.count:
    inc ax
    mov [0x402],ax
    mov word [es:0],0          ; invalidate stale saved completion immediately
    xor ax,ax
    mov es,ax
    in al,0xa0
    and ax,2
    mov [0x404],ax
    mov di,0x408
    call probe                ; every model begins in mono graphics
    cmp word [0x404],0
    je complete
%assign mode_index 0
%rep 6
%if mode_index = 0
    put 0x60,0x80
%elif mode_index = 2
    put 0x60,0xc0
%elif mode_index = 4
    put 0x60,0xe0
%else
    put 0x60,0
%endif
    call probe
%assign mode_index mode_index+1
%endrep
complete:
    cmp word [0x900],0
    jne failed
    cmp sp,0x3000
    jne failed
    mov word [0x400],0x600d
    jmp display

; Literal entry SP, updated SP, SS, mapped physical address, FLAGS, encoding.
; Capture the actual stack bytes, POP result, both SP values and full FLAGS
; before any comparison changes FLAGS. Continue after failures so both encodings
; and every phase produce observations even on an incorrect emulator.
%macro observe 6
    push word %5
    popf
    mov ax,%3
    mov ss,ax
    mov sp,%1
%if %6 = 0
    db 0x54
%else
    db 0xff,0xf4
%endif
    mov [di+4],sp
    mov bx,%4
    mov dx,[bx]
    pop ax
    mov [di],dx
    mov [di+2],ax
    mov [di+6],sp
    mov ax,0
    mov ss,ax
    mov sp,0x2ffe             ; preserve the probe's CALL return address
    pushf
    pop dx
    mov [di+8],dx
    cmp word [di],%2
    jne %%bad
    cmp word [di+2],%2
    jne %%bad
    cmp word [di+4],%2
    jne %%bad
    cmp word [di+6],%1
    jne %%bad
    cmp dx,%5
    je %%done
%%bad:
    mov word [0x900],1
%%done:
    add di,10
%endmacro

probe:
    xor ax,ax
    cmp word [0x404],0
    je .mode
    in al,0x60
.mode:
    mov [di],ax
    add di,2
%assign encoding 0
%rep 2
%assign flag_index 0
%rep 2
%if flag_index = 0
%assign flags 0xf002
%else
%assign flags 0xfcd7            ; opposite arithmetic/DF bits, no IRQ or trap
%endif
    observe 0x2000,0x1ffe,0x0020,0x21fe,flags,encoding
    observe 0x2001,0x1fff,0x0020,0x21ff,flags,encoding
    observe 0x0002,0x0000,0x0020,0x0200,flags,encoding
    observe 0x0000,0xfffe,0xf100,0x0ffe,flags,encoding
%assign flag_index flag_index+1
%endrep
%assign encoding encoding+1
%endrep
    inc word [0x406]          ; fresh completed-phase marker
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
    mov cx,571
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
