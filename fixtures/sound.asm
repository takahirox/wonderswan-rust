; SPDX-License-Identifier: CC0-1.0
; Original Issue #19 audio probes. Deterministic PCM, no external samples/code.
; SOUND_PATH: 0 = SDMA Channel 2; 1 = SDMA Hyper Voice; 2 = CPU Hyper Voice.
bits 16
cpu 186
org 0

%macro put_port 2
    mov al,%2
    out %1,al
%endmacro
%macro check_port 2
    in al,%1
    cmp al,%2
    jne failed
%endmacro

start:
    cli
    cld
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,0x3000
    in al,0xa0
    test al,2
    jnz color
    mov word [0x400],0x4d4f     ; mono: skip color-only I/O and remain silent
    hlt
color:
    put_port 0x60,0x80
    put_port 0x91,8             ; headphones, no speaker
%if SOUND_PATH < 2
    ; A finite 2-byte transfer first checks live counters and auto-disable.
    mov word [0x1000],0xe020
    put_port 0x4a,0
    put_port 0x4b,0x10
    put_port 0x4c,0xf0          ; source high reserved bits masked
    put_port 0x4d,255           ; unused high byte never stores state
    put_port 0x4e,2
    put_port 0x4f,0
    put_port 0x50,0xf0          ; length high reserved bits masked
    put_port 0x51,255
    check_port 0x4c,0
    check_port 0x4d,0
    check_port 0x50,0
    check_port 0x51,0
%if SOUND_PATH = 0
    put_port 0x90,0x20         ; legacy Channel 2 PCM; wavetable enable unnecessary
    put_port 0x94,5            ; full left and right
    put_port 0x52,0xa3         ; 24 kHz, reserved bit masked, Channel 2
%else
    put_port 0x6a,0x89         ; signed, half amplitude, 24 kHz
    put_port 0x6b,0x70         ; reset next left, mono both
    put_port 0x52,0xb3         ; 24 kHz, reserved bit masked, Hyper Voice
%endif
wait_dma:
    in al,0x52
    test al,0x80
    jnz wait_dma
    check_port 0x4a,2
    check_port 0x4b,0x10
    check_port 0x4e,0
%if SOUND_PATH = 0
    check_port 0x89,0xe0
%else
    check_port 0x68,0xe0
    check_port 0x69,0xe0
%endif
    mov word [0x402],1         ; finite-transfer assertions passed
    ; Generate a 32-byte, 750 Hz square wave in RAM: sixteen 20, sixteen E0.
    mov di,0x1000
    mov ax,0x2020
    mov cx,8
    rep stosw
    mov ax,0xe0e0
    mov cx,8
    rep stosw
    put_port 0x4a,0
    put_port 0x4e,32
%if SOUND_PATH = 0
    put_port 0x52,0x8b         ; 24 kHz, repeat, Channel 2
%else
    put_port 0x52,0x9b         ; 24 kHz, repeat, Hyper Voice
%endif
%else
    ; Manual input always alternates even when DMA layout is mono-both.
    put_port 0x6a,0x89
    put_port 0x6b,0x70
    put_port 0x69,0x20
    put_port 0x69,0xe0
    call delay
    check_port 0x64,0
    check_port 0x65,0x10        ; +4096
    check_port 0x66,0
    check_port 0x67,0xf0        ; -4096
    put_port 0x6a,0            ; disable converter; direct output remains usable
    put_port 0x64,0
    put_port 0x65,0x20
    put_port 0x66,0
    put_port 0x67,0xe0
    check_port 0x65,0x20
    check_port 0x67,0xe0
    mov word [0x402],1         ; manual 8/16-bit assertions passed
%endif
    mov word [0x400],0x600d
    mov bx,0x00f0              ; green = register assertions passed
    call display
%if SOUND_PATH < 2
    hlt                       ; SDMA continues while CPU is halted
%else
manual_pcm:
    put_port 0x6a,0            ; direct 16-bit CPU output
    put_port 0x65,0x20
    put_port 0x67,0xe0
    call delay
    put_port 0x65,0xe0
    put_port 0x67,0x20
    call delay
    put_port 0x6a,0x89         ; alternate with CPU 8-bit input/conversion
    put_port 0x6b,0x70
    put_port 0x69,0x20
    put_port 0x69,0xe0
    call delay
    put_port 0x69,0xe0
    put_port 0x69,0x20
    call delay
    jmp manual_pcm
%endif
failed:
    mov word [0x400],0x0bad
    mov bx,0x0f00              ; red = failed cartridge assertions
    call display
    hlt
display:
    mov [0xfe02],bx
    mov di,0x2000
    mov ax,0x00ff
    mov cx,8
    rep stosw
    put_port 7,0
    put_port 0,1
    put_port 0x14,1
    ret
delay:
    mov cx,80
wait_sample:
    loop wait_sample
    ret
