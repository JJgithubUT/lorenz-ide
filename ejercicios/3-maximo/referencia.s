; R3 = el mayor de R1 y R2
        SLT   R4, R1, R2        ; R4 = 1 si R1 < R2
        LI    R5, 0
        BNE   R4, R5, segundo
        ADDI  R3, R1, 0
        HLT
segundo: ADDI R3, R2, 0
        HLT
