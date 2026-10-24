; SPDX-License-Identifier: CC0-1.0
; Original Issue #53 RTC transport probe. No imported code, assets or firmware.
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
    mov cx,466
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

; Original bounded CPU polling, using CA/CB only. Records are independent of
; the emulator command table. READY is consumed before ACTIVE is checked.
%macro transaction 2
    mov bl,%1
    call poll
    cmp word [di],%2
    jne %%bad
    cmp word [di+2],0x80 | (%1 & 15)
    jne %%bad
    cmp word [di+4],%1 & 15
    jne %%bad
    cmp word [di+6],(%1 & 1) * 255
    jne %%bad
    cmp word [di+8],%1 & 0x7f
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
    transaction 0x10,1
    transaction 0x11,1
    transaction 0x12,1
    transaction 0x13,1
    transaction 0x14,7
    transaction 0x15,7
    transaction 0x16,3
    transaction 0x17,3
    transaction 0x18,2
    transaction 0x19,2
    transaction 0x1a,2
    transaction 0x1b,2
    transaction 0x93,1
    inc word [0x406]
    ret

poll:
    mov word [di],0
    mov word [di+6],0
    test bl,1
    jz .write
    mov word [di+6],255
    jmp .start
.write:
    inc word [di]             ; preloaded first write is one host byte
.start:
    mov al,0x5a
    out 0xcb,al
    mov al,bl
    out 0xca,al
    in al,0xca
    xor ah,ah
    mov [di+8],ax             ; fresh command, READY cleared (including 93)
    mov bp,4096
.wait:
    in al,0xca
    mov dl,al
    xor dh,dh
    test al,0x90
    jz .done
    test al,0x80
    jz .active
    test bl,1
    jz .send
    in al,0xcb                ; consume terminal data before testing ACTIVE
    and [di+6],al
    inc word [di]
    jmp .active
.send:
    mov al,0xa5
    out 0xcb,al               ; terminal acknowledgment is not another byte
    test dl,0x10
    jz .done
    inc word [di]
.active:
    test dl,0x10
    jz .done
    dec bp
    jnz .wait
    mov dx,0xffff            ; timeout never looks like a completion
.done:
    mov [di+2],dx
    mov cx,64
.settle:
    loop .settle
    in al,0xca
    xor ah,ah
    mov [di+4],ax             ; no phantom READY after terminal consumption
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
    mov cx,466
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
