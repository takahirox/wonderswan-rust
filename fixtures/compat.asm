; SPDX-License-Identifier: CC0-1.0
; Original hardware probes, authored for this repository. No firmware/assets.
; NASM emits code for the final 64 KiB bank; build.py creates the cartridge.
bits 16
cpu 186
org 0
%ifndef EEPROM_BITS
%define EEPROM_BITS 0
%endif
%ifndef EXTENDED
%define EXTENDED 0
%endif
%ifndef VIDEO_MODE
%define VIDEO_MODE 0xc0
%endif
start:
    cli
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,0x2000
    cld
    ; Probe ROM bank 0 through the mapper, independently of the reset mapping.
    mov al,0
    out 0xc2,al
    mov ax,0x2000
    mov es,ax
    mov al,[es:0]
    mov [0x408],al
    xor ax,ax
    mov es,ax
%if EXTENDED
    mov al,15
    out 0xcc,al
    mov al,5
    out 0xcd,al
    in al,0xcd
    mov [0x40a],al
    mov al,0
    out 0xcf,al
    out 0xc1,al
    mov al,1
    out 0xce,al
    mov ax,0x1000
    mov es,ax
    mov al,[es:0]
    mov [0x40b],al
    xor ax,ax
    mov es,ax
    out 0xce,al
    mov al,0x13
    out 0xca,al
 .rtc_ready:
    in al,0xca
    test al,0x80
    jz .rtc_ready
    in al,0xcb
    mov [0x40c],al
%endif
%if EEPROM_BITS
    ; Enable writes, read/increment/save word zero. Poll controller status.
    mov ax,(1 << (EEPROM_BITS+2)) | (3 << (EEPROM_BITS-2))
    out 0xc6,ax
    mov al,0x40
    out 0xc8,al
    call eeprom_wait
    mov ax,(1 << (EEPROM_BITS+2)) | (2 << EEPROM_BITS)
    out 0xc6,ax
    mov al,0x10
    out 0xc8,al
    call eeprom_wait
    in ax,0xc4
    cmp ax,0xffff
    jne .increment
    xor ax,ax
.increment:
    inc ax
    mov [0x400],ax
    out 0xc4,ax
    mov ax,(1 << (EEPROM_BITS+2)) | (1 << EEPROM_BITS)
    out 0xc6,ax
    mov al,0x20
    out 0xc8,al
    call eeprom_wait
%else
    mov al,0
    out 0xc1,al
    mov ax,0x1000
    mov es,ax
    mov al,[es:0]
    cmp al,0xff
    jne .increment
    xor al,al
.increment:
    inc al
    mov [es:0],al
    mov [0x400],al
%if EXTENDED
    add al,0x40
    mov bl,al
    mov al,1
    out 0xc1,al
    mov [es:0],bl
    xor al,al
    out 0xc1,al
%else
    mov al,[es:0x8000]              ; 32 KiB SRAM mirror
    mov [0x40d],al
%endif
    xor ax,ax
    mov es,ax
%endif
    ; Signed multiply, carry arithmetic, rotate, far call/return and INT/IRET.
    mov ax,-7
    mov bx,3
    imul bx
    add ax,21
    stc
    adc ax,4
    rol ax,1
    mov [0x406],ax                  ; 10
    call 0xf000:far_return
    mov word [0x80],software_irq
    mov word [0x82],0xf000
    int 0x20                       ; software handler writes 0x40e
    ; Color screen 1 green, windowed screen 2 red, high sprite blue.
    in al,0xa0
    test al,2
    jz .mono
    mov al,VIDEO_MODE
    out 0x60,al
    mov di,0x4000
    mov cx,3
.tile:
    push cx
    mov cx,8
.row:
%if VIDEO_MODE == 0xe0
    mov ax,0x1111
    stosw
    stosw
%else
    mov ax,0x00ff
    stosw
    xor ax,ax
    stosw
%endif
    loop .row
    pop cx
    loop .tile
    mov word [0xfe02],0x00f0        ; palette 0 green
    mov word [0xfe22],0x0f00        ; palette 1 red
    mov word [0xff82],0x000f        ; sprite palette 12 blue
    jmp .display
.mono:
    mov di,0x2000
    mov ax,0x00ff
    mov cx,24
    rep stosw
    mov ax,0x3210
    out 0x20,ax
    out 0x22,ax
    out 0x38,ax
    mov ax,0x7654
    out 0x1c,ax
    mov ax,0xfedc
    out 0x1e,ax
.display:
    mov di,0x1000
    mov ax,0x0201
    mov cx,1024
    rep stosw
    mov word [0x1800],0x2802        ; palette 12, front of screen 2
    mov word [0x1802],0x1010        ; x=16,y=16
    mov al,12
    out 4,al
    xor al,al
    out 5,al
    mov al,1
    out 6,al
    mov al,0x21
    out 7,al
    mov ax,0x0808
    out 8,ax
    mov ax,0x1f1f
    out 10,ax
    mov al,0x27
    out 0,al
    mov al,1
    out 0x14,al
    ; All four wavetable channels, a repeating square wave.
    mov di,0x1c00
    mov cx,4
.wave:
    push cx
    xor ax,ax
    mov cx,4
    rep stosw
    mov ax,0xffff
    mov cx,4
    rep stosw
    pop cx
    loop .wave
    mov al,0x70
    out 0x8f,al
    mov ax,0x07a0
    out 0x80,ax
    out 0x82,ax
    out 0x84,ax
    out 0x86,ax
    mov ax,0xffff
    out 0x88,ax
    out 0x8a,ax
    mov al,15
    out 0x90,al
    mov al,8
    out 0x91,al
    ; Interrupt vectors 24 (line), 26 (VBlank), 27 (HBlank timer).
    mov word [0x90],hardware_irq
    mov word [0x92],0xf000
    mov word [0x98],hardware_irq
    mov word [0x9a],0xf000
    mov word [0x9c],hardware_irq
    mov word [0x9e],0xf000
    mov al,0x20
    out 0xb0,al
    mov al,0xd0
    out 0xb2,al
    mov al,40
    out 3,al
    mov ax,16
    out 0xa4,ax
    mov al,3
    out 0xa2,al
    sti
.loop:
    hlt
    mov al,0x20
    out 0xb5,al
    in al,0xb5
    mov [0x404],al
    jmp .loop
far_return:
    retf
software_irq:
    mov word [0x40e],0x1234
    iret
hardware_irq:
    push ax
    inc word [0x402]
    mov al,0xd0
    out 0xb6,al
    pop ax
    iret
%if EEPROM_BITS
eeprom_wait:
    in al,0xc8
    test al,2
    jz eeprom_wait
    ret
%endif
