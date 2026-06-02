; R3 = |R1 - R2|
        SLT   R4, R1, R2        ; R4 = 1 si R1 < R2
        LI    R5, 0
        BNE   R4, R5, menor
        SUB   R3, R1, R2
        HLT
menor:  SUB   R3, R2, R1
        HLT
