; La misma suma de vectores en la CPU: c[i] = a[i] + b[i] para 16 elementos.
; a esta en 0x4000, b en 0x4010 y c en 0x4020. Compara con kernels/suma-vectores.kgpu.
        LI    R3, 256          ; R3 = 0x4000 (256 * 64, duplicando seis veces)
        ADD   R3, R3, R3
        ADD   R3, R3, R3
        ADD   R3, R3, R3
        ADD   R3, R3, R3
        ADD   R3, R3, R3
        ADD   R3, R3, R3
        ADDI  R4, R3, 0        ; puntero a a
        ADDI  R5, R3, 16       ; puntero a b
        ADDI  R6, R3, 31
        ADDI  R6, R6, 1        ; puntero a c (0x4020)
        LI    R2, 16           ; elementos que faltan
bucle:  LW    R1, [R4+0]
        LW    R7, [R5+0]
        ADD   R1, R1, R7
        SW    R1, [R6+0]
        ADDI  R4, R4, 1
        ADDI  R5, R5, 1
        ADDI  R6, R6, 1
        ADDI  R2, R2, -1
        BNE   R2, R0, bucle
        HLT

        .org  0x4000
        .dw   1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        .dw   10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160
