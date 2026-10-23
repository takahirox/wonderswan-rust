; SPDX-License-Identifier: CC0-1.0
; Original Issue #27 CPU/raster probe; no external ROM, firmware or test code.
bits 16
cpu 186
org 0
%macro put 2
    mov al,%2
    out %1,al
%endmacro
%macro check 2
    in al,%1
    cmp al,%2
    jne failed
%endmacro
start:
    cli
    xor ax,ax
    mov ds,ax
    mov ss,ax
    mov sp,0x3000
    mov ax,0x1000
    mov es,ax                     ; own SRAM execution observations
    mov word [es:8],0
    in al,0xa0
    and ax,2
    mov bp,ax                     ; nonzero for color-capable model
    xor bx,bx
    test bp,bp
    jz .model
    in al,0x62
    mov bl,al                     ; Crystal has bit 7
.model:
    mov [0x40a],bx
    put 0x16,156                  ; independent byte OUT/IN
    put 0x17,153
    check 0x16,156
    mov ah,153
    call check_sync
    put 0x17,155                  ; changing sync must not change total
    check 0x16,156
    mov ah,155
    call check_sync
    put 0x17,153
    put 3,144
    put 0xb2,0xf0                ; observe latched IRQs with IF clear
    put 0xa4,1
    put 0xa5,0
    put 0xa6,1
    put 0xa7,0
    put 0xa2,0x0f                ; both timers repeat every source event
    mov dx,156
    call observe_frame
    mov [0x402],cx               ; checked 157-line progression
    mov [es:2],cx
    mov ax,0xb1b4                ; total=180, STN sync=177
    out 0x16,ax                  ; actual immediate word OUT/IN
    in ax,0x16
    call check_word
    mov dx,0x16
    mov ax,0xb1b4
    out dx,ax                    ; actual DX word OUT/IN
    in ax,dx
    call check_word
    mov dx,180
    call observe_frame
    mov [0x404],cx               ; checked 181-line progression
    mov [es:4],cx
    mov word [0x400],0x600d
    mov word [es:0],0x600d
    test bp,bp
    jz .mono
    put 0x60,0x80
    mov word [0xfe02],0x00f0      ; green success
    jmp display
.mono:
    mov ax,0x7654                ; shade 170 success
    out 0x20,ax
    jmp display
failed:
    mov word [0x400],0x0bad
    mov word [es:0],0x0bad
    test bp,bp
    jz .mono
    put 0x60,0x80
    mov word [0xfe02],0x0f00
    jmp display
.mono:
    mov ax,0x7653                ; shade 187 failure
    out 0x20,ax
 display:
    put 0x1c,0x10
    put 0x1d,0x32
    put 0x1e,0x54
    put 0x1f,0x76
    push es
    xor ax,ax
    mov es,ax
    mov di,0x2000
    mov ax,0x00ff
    mov cx,8
    cld
    rep stosw
    pop es
    put 7,0
    put 0,1
    put 0x14,1
    cmp word [0x400],0x600d
    jne .halt
    mov word [0x406],180
    mov word [es:6],180
    put 0xb2,0
    put 0xb6,255
    put 0xb5,0x20
.runtime:
    in al,0xb5
    and al,2                     ; Right selects shorter normal total
    mov dx,180
    jz .program
    mov dx,156
.program:
    cmp dx,[0x406]
    je .runtime
    mov al,dl
    out 0x16,al
    sub al,3
    out 0x17,al
    in al,0x16
    cmp al,dl
    jne failed
    mov [0x406],dx
    mov [es:6],dx
    inc word [0x408]
    mov ax,[0x408]
    mov [es:8],ax
    jmp .runtime
.halt:
    hlt
    jmp .halt
check_sync:
    in al,0x17
    cmp word [0x40a],0
    je .stn
    xor ah,ah
.stn:
    cmp al,ah
    jne failed
    ret
check_word:
    cmp word [0x40a],0
    je .stn
    cmp ax,180
    jne failed
    ret
.stn:
    cmp ax,0xb1b4
    jne failed
    ret
observe_frame:
    ; Begin at a real line-zero edge. Every subsequent line must increment by
    ; one until T, then wrap to zero. Polling runs well below 256 clocks/line.
.wait_nonzero:
    in al,2
    test al,al
    jz .wait_nonzero
.wait_zero:
    in al,2
    test al,al
    jnz .wait_zero
    put 0xb6,255
    xor bx,bx
    mov cx,1
.next:
    in al,2
    cmp al,bl
    je .next
    cmp bx,dx
    je .wrap
    inc bx
    cmp al,bl
    jne failed
    inc cx
    jmp .next
.wrap:
    test al,al
    jnz failed
    in al,0xb4
    cmp al,0xf0                  ; line144, VBlank, H timer, V timer
    jne failed
    mov [es:10],al
    check 0xa8,1
    check 0xaa,1
    ret
