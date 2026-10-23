; SPDX-License-Identifier: CC0-1.0
; Original relative bus timing probe. No firmware or third-party code/data.
; Each 4096-transfer odd loop adds 4096 clocks = 16 HBlank timer ticks.
; Two separately quantized elapsed observations allow +/-2 ticks. Byte and
; aligned-word loops have equal bus costs, also compared within +/-2 ticks.
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
    mov cx,32
    rep stosw
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
    mov word [es:2],0            ; invalidate saved completion before execution
    xor ax,ax
    mov es,ax
    ; Copy the same calibrated loop into internal RAM on every model. All
    ; samples use identical code addresses. No ROM width/prefetch comparison.
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
    out 0xa2,al                  ; HBlank one-shot, no IRQ delivery (CLI)

%macro sample 5                 ; marker, code bytes, BX, DX, description
    mov word [0x2000+transfer-body],%2
    mov bx,%3
    mov dx,%4
    call 0:0x2000
    mov [%1],ax
%endmacro
    sample 0x404,0x078a,0x800,0,read_byte
    sample 0x406,0x078b,0x800,0,read_even
    sample 0x408,0x078b,0x801,0,read_odd
    sample 0x40a,0x0788,0x800,0,write_byte
    sample 0x40c,0x0789,0x800,0,write_even
    sample 0x40e,0x0789,0x801,0,write_odd
    ; BC/BD is an internal word command register; BB/BC are valid byte lanes.
    ; No BE control access occurs, so no EEPROM operation or busy side effect.
    sample 0x410,0x90ec,0,0xbc,input_byte
    sample 0x412,0x90ed,0,0xbc,input_even
    sample 0x414,0x90ed,0,0xbb,input_odd
    sample 0x416,0x90ee,0,0xbc,output_byte
    sample 0x418,0x90ef,0,0xbc,output_even
    sample 0x41a,0x90ef,0,0xbb,output_odd

    mov si,0x404
    mov cx,4
.check:
    mov ax,[si]
    sub ax,[si+2]                ; byte minus aligned: -2..2
    add ax,2
    cmp ax,4
    ja failed
    mov ax,[si+4]
    sub ax,[si+2]                ; odd minus aligned: 14..18 ticks
    cmp ax,14
    jb failed
    cmp ax,18
    ja failed
    add si,6
    loop .check
    mov word [0x402],0x600d
    jmp publish
failed:
    mov word [0x402],0x0bad
publish:
    mov ax,0x1000
    mov es,ax
    mov si,0x400
    xor di,di
    mov cx,14
    rep movsw                    ; CPU-written RAM results mirrored to SRAM
.done:
    hlt
    jmp .done

body:
    mov cx,4096
    in ax,0xa8
    mov bp,ax
transfer:
    mov ax,[bx]                  ; replaced between samples, never during one
    loop transfer
    in ax,0xa8
    sub bp,ax
    mov ax,bp
    retf
body_end:
