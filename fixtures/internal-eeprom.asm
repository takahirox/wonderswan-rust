; SPDX-License-Identifier: CC0-1.0
; Original Issues #21/#91 CPU probe. No copied test code, firmware or assets.
bits 16
cpu 186
org 0
start:
    cli
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,0x3000
    cld
    ; Every model starts in mono command mode. Firmware-free header handoff.
    in ax,0xbe
%if PROTECTED
    cmp ax,0x82
%else
    cmp ax,2
%endif
    jne failed
    ; Read owner/settings through actual byte command and word data I/O.
    mov al,0xb0
    out 0xbc,al
    mov al,1
    out 0xbd,al
    mov ax,0x10
    out 0xbe,ax
    in ax,0xbe
    test ax,1
    jnz failed                     ; READ DONE must first clear
    call wait_ready
    in ax,0xba
    mov [0x406],ax                 ; original settings word
    ; Word zero is reserved for direct ERASE; counter moves to word one.
    mov ax,0x1234
    mov dx,0x140
    call write_word
    ; Read/increment shared word one, common across cartridges.
    mov ax,0x181
    call read_word
    cmp ax,0xffff
    jne .increment
    xor ax,ax
.increment:
    inc ax
    mov [0x402],ax
    mov dx,0x141
    call write_word
    mov ax,0x181
    call read_word
    cmp ax,[0x402]
    jne failed
    ; WDS must prevent a write; WEN must allow it again.
    mov ax,0x100
    out 0xbc,ax
    mov ax,0x40
    out 0xbe,ax
    call wait_ready
    mov ax,0xdead
    mov dx,0x141
    call write_word
    mov ax,0x181
    call read_word
    cmp ax,[0x402]
    jne failed
    ; WDS also prevents a direct word-zero ERASE.
    mov ax,0x1c0
    out 0xbc,ax
    mov al,0x40
    out 0xbe,al
    call wait_ready
    mov ax,0x180
    call read_word
    cmp ax,0x1234
    jne failed
    mov ax,0x130
    out 0xbc,ax
    mov ax,0x40
    out 0xbe,ax
    call wait_ready
    ; Distinguish settings protection at handoff from the write-enable latch.
    mov ax,[0x406]
    xor ax,0xffff
    mov dx,0x170
    call write_word
    mov ax,0x1b0
    call read_word
%if PROTECTED
    cmp ax,[0x406]
    jne failed
%else
    xor ax,0xffff
    cmp ax,[0x406]
    jne failed
    ; Restore the original settings word before protecting it.
    mov ax,[0x406]
    mov dx,0x170
    call write_word
%endif
    ; Set protection twice, with a nonzero reserved BF high byte. Not an abort.
    mov ax,0x5a80
    out 0xbe,ax
    out 0xbe,ax
    mov ax,0x55aa
    mov dx,0x16f                  ; final shared word (byte 5E)
    call write_word
    in ax,0xba                    ; writes leave the separate read buffer alone
%if PROTECTED
    cmp ax,[0x406]
%else
    xor ax,0xffff
    cmp ax,[0x406]
%endif
    jne failed
    mov ax,0x1af
    call read_word
    cmp ax,0x55aa
    jne failed
    mov ax,0xdead
    mov dx,0x170                  ; first settings word (byte 60)
    call write_word
    mov ax,0x1b0
    call read_word
    cmp ax,[0x406]
    jne failed
    in ax,0xbe
    cmp ax,0x83
    jne failed
    ; Direct ERASE at word zero: READY busy, DONE/read buffer unchanged.
    mov ax,0x180
    call read_word
    cmp ax,0x1234
    jne failed
    mov ax,0x1c0
    out 0xbc,ax
    mov ax,0x5a4f                 ; nonzero BF and reserved low BE bits
    out 0xbe,ax
    in ax,0xbe
    cmp ax,0x81
    jne failed
    in ax,0xba
    cmp ax,0x1234
    jne failed
    ; In-flight address/data changes cannot redirect the accepted erase.
    mov ax,0xdead
    out 0xba,ax
    out 0xbc,ax
    mov al,0x80
    out 0xbe,al                   ; protection is not an abort
    call wait_ready
    in ax,0xba
    cmp ax,0x1234
    jne failed
    mov ax,0x180
    call read_word
    cmp ax,0xffff
    jne failed
    mov [0x40a],ax
    ; Color physical capacity persists through mono -> Color -> mono -> Color.
    in al,0xa0
    test al,2
    jz passed
    mov al,0x80
    out 0x60,al
    mov ax,0x1801
    call read_word
    cmp ax,[0x402]
    jne failed
    ; Same supported ERASE in ten-bit mode, leaving persisted bytes FFFF.
    mov ax,0x5678
    mov dx,0x1400
    call write_word
    mov ax,0x1800
    call read_word
    cmp ax,0x5678
    jne failed
    mov ax,0x1c00
    out 0xbc,ax
    mov ax,0x40
    out 0xbe,ax
    in ax,0xbe
    cmp ax,0x81
    jne failed
    xor al,al
    out 0x60,al                   ; mode changes cannot reinterpret pending erase
    call wait_ready
    in ax,0xba
    cmp ax,0x5678
    jne failed
    mov ax,0x180
    call read_word
    cmp ax,0xffff
    jne failed
    mov al,0x80
    out 0x60,al
    mov ax,0x1bff                  ; physical final word, beyond mono address range
    call read_word
    mov [0x408],ax
    mov ax,0xdead
    mov dx,0x17ff
    call write_word               ; protected even at the final Color word
    mov ax,0x1bff
    call read_word
    cmp ax,[0x408]
    jne failed
    xor al,al
    out 0x60,al
    mov ax,0x181
    call read_word
    cmp ax,[0x402]
    jne failed
    mov al,0x80
    out 0x60,al
    in ax,0xbe
    cmp ax,0x83
    jne failed
passed:
    mov word [0x400],0x600d
    mov bx,0x00f0
    jmp display
failed:
    mov word [0x400],0x0bad
    mov bx,0x0f00
display:
    in al,0xa0
    test al,2
    jz .mono
    mov al,0x80
    out 0x60,al
    mov [0xfe02],bx
    jmp .tiles
.mono:
    mov ax,0x3210
    cmp bx,0x00f0
    je .palette
    mov ax,0x3200
.palette:
    out 0x20,ax
    mov ax,0x7654
    out 0x1c,ax
    mov ax,0xfedc
    out 0x1e,ax
.tiles:
    mov di,0x2000
    mov ax,0x00ff
    mov cx,8
    rep stosw
    xor al,al
    out 7,al
    mov al,1
    out 0,al
    out 0x14,al
    hlt
read_word:
    out 0xbc,ax
    mov ax,0x10
    out 0xbe,ax
    call wait_ready
    in ax,0xba
    ret
write_word:
    out 0xba,ax
    mov ax,dx
    out 0xbc,ax
    mov ax,0x20
    out 0xbe,ax
    call wait_ready
    ret
wait_ready:
    in ax,0xbe
    test ax,2
    jz wait_ready
    ret
