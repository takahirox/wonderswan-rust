; SPDX-License-Identifier: CC0-1.0
; Original Issue #91 diagnostic probes, not physical-console tests.
; Each image intentionally stops on one rejected CPU OUT, after a real WRITE.
bits 16
cpu 186
org 0
    cli
    xor ax,ax
    mov ds,ax
    mov ss,ax
    mov sp,0x3000
    mov bx,0x1c0                  ; start/opcode ERASE, six-bit address
    mov dx,0x140                  ; WRITE zero
    mov si,0x100                  ; opcode zero/start
    mov di,0x10                   ; subcommand shift
    mov bp,63                    ; final physical word
    in al,0xa0
    test al,2
    jz .configured
    mov al,0x80
    out 0x60,al
    mov bx,0x1c00                 ; ten-bit address on Color/Crystal
    mov dx,0x1400
    mov si,0x1000
    mov di,0x100
    mov bp,1023
.configured:
    mov ax,0x1234
    out 0xba,ax
    mov ax,dx
    out 0xbc,ax
    mov al,0x20
    out 0xbe,al
    call wait_ready
    mov word [0x400],0x91ee       ; host verifies we reached the failing OUT
    mov ax,bx
%if CASE == 1
    or ax,0x2f                   ; last user word, byte 5E
%elif CASE == 2
    or ax,0x30                   ; first protected word, byte 60
%elif CASE == 3
    or ax,bp                     ; physical final word
%elif CASE == 4
    mov ax,si
    or ax,di                     ; candidate WRAL
%elif CASE == 5
    mov ax,si
    shl di,1
    or ax,di                     ; candidate ERAL
%elif CASE == 7
    xor ax,si                    ; missing start bit
%elif CASE == 8
    out 0xbc,ax                  ; start valid ERASE, immediately request again
    mov al,0x40
    out 0xbe,al
    mov ax,bx
%endif
    out 0xbc,ax
%if CASE == 4
    mov al,0x20
%elif CASE == 6
    mov al,0x60                  ; mixed WRITE/ERASE controls
%else
    mov al,0x40
%endif
    out 0xbe,al                  ; expected explicit emulator diagnostic
    mov word [0x400],0x0bad      ; reaching this instruction is a test failure
    hlt
wait_ready:
    in ax,0xbe
    test al,2
    jz wait_ready
    ret
