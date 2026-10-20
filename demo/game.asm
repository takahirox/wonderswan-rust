; SPDX-License-Identifier: CC0-1.0
; Beacon Trail — original WonderSwan cartridge, no firmware calls.
bits 16
cpu 186
org 0
%define X 0x0400
%define Y 0x0402
%define SCORE 0x0404
%define DELAY 0x0406
%define TX 0x0408
%define TY 0x040a

start:
    cli
    cld
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov di, ax
    mov cx, 0x2000
    rep stosw                    ; initialize all 16 KiB of common RAM
    mov sp, 0x3f00
    mov al, 0
    out 0x00, al                 ; screen off while setting up
    out 0x10, al
    out 0x11, al
    mov al, 1
    out 0x07, al                 ; Screen 1 map at 0800
    out 0x14, al                 ; LCD enabled
    mov ax, 0xa7f0               ; shade indices 0..3: white, black, gray, dark
    out 0x1c, ax
    mov ax, 0x3210               ; palette 0 maps color indices to shades
    out 0x20, ax
    push cs
    pop ds
    mov si, tiles
    mov di, 0x2000
    mov cx, 1024
    rep movsw
    xor ax, ax
    mov ds, ax

new_game:
    xor ax, ax
    mov di, 0x0800
    mov cx, 1024
    rep stosw
    mov word [X], 3
    mov word [Y], 8
    mov word [SCORE], 0
    mov word [DELAY], 0
    mov di, 0x0800 + 64 + 2*2
    mov si, title
    call text
    mov di, 0x0800 + 64 + 21*2
    mov si, progress
    call text
    mov di, 0x0800 + 16*64 + 3*2
    mov si, hint
    call text
    mov bx, 0x0800 + 3*64 + 2
    mov cx, 26
.border_horizontal:
    mov word [bx], 3
    mov word [bx+12*64], 3
    add bx, 2
    loop .border_horizontal
    mov bx, 0x0800 + 4*64 + 2
    mov cx, 11
.border_vertical:
    mov word [bx], 3
    mov word [bx+25*2], 3
    add bx, 64
    loop .border_vertical
    call target
    call player
    mov al, 1
    out 0x00, al

frame:
.outside_vblank:
    in al, 0x02
    cmp al, 144
    jae .outside_vblank
.wait_vblank:
    in al, 0x02
    cmp al, 144
    jb .wait_vblank
    mov al, 0x40
    out 0xb5, al
    nop
    nop
    in al, 0xb5
    test al, 2                    ; Start: restart through cartridge code
    jz .not_start
    jmp new_game
.not_start:
    cmp word [SCORE], 3
    je frame
    mov al, 0x20
    out 0xb5, al
    nop
    nop
    in al, 0xb5
    and al, 15
    jnz .held
    mov word [DELAY], 0
    jmp frame
.held:
    cmp word [DELAY], 0
    je .move
    dec word [DELAY]
    jmp frame
.move:
    mov word [DELAY], 7           ; repeat every 8 frames (~9 steps/second)
    push ax
    call position
    mov word [bx], 0
    pop ax
    test al, 1
    jz .right
    cmp word [Y], 4
    jbe .right
    dec word [Y]
.right:
    test al, 2
    jz .down
    cmp word [X], 25
    jae .down
    inc word [X]
.down:
    test al, 4
    jz .left
    cmp word [Y], 14
    jae .left
    inc word [Y]
.left:
    test al, 8
    jz .collect
    cmp word [X], 2
    jbe .collect
    dec word [X]
.collect:
    mov ax, [X]
    cmp ax, [TX]
    jne .draw
    mov ax, [Y]
    cmp ax, [TY]
    jne .draw
    inc word [SCORE]
    mov ax, [SCORE]
    add ax, '0'
    mov [0x0800 + 64 + 21*2], ax
    cmp word [SCORE], 3
    je .won
    call target
.draw:
    call player
    jmp frame
.won:
    call player
    mov di, 0x0800 + 10*64 + 9*2
    mov si, win
    call text
    jmp frame

; BX = tilemap address of (X,Y), using ordinary 16-bit CPU arithmetic.
position:
    mov bx, [Y]
    add bx, bx
    add bx, bx
    add bx, bx
    add bx, bx
    add bx, bx
    add bx, [X]
    add bx, bx
    add bx, 0x0800
    ret
player:
    call position
    mov word [bx], 1
    ret
target:
    mov word [TX], 9
    mov word [TY], 8
    cmp word [SCORE], 0
    je .draw
    mov word [TY], 5
    cmp word [SCORE], 1
    je .draw
    mov word [TX], 18
.draw:
    mov ax, [X]
    push ax
    mov ax, [Y]
    push ax
    mov ax, [TX]
    mov [X], ax
    mov ax, [TY]
    mov [Y], ax
    call position
    mov word [bx], 2
    pop ax
    mov [Y], ax
    pop ax
    mov [X], ax
    ret
text:
    mov al, [cs:si]
    inc si
    cmp al, 0
    je .done
    mov ah, 0
    stosw
    jmp text
.done:
    ret

title: db 'BEACON TRAIL', 0
progress: db '0/3', 0
hint: db 'ARROWS MOVE  ENTER RESET', 0
win: db 'ALL FOUND!', 0

tiles:
    incbin "tiles.bin"
