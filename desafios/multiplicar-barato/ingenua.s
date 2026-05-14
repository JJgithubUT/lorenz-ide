        LI    R3, 0
        LI    R5, 0
bucle:  BEQ   R2, R5, fin
        ADD   R3, R3, R1
        ADDI  R2, R2, -1
        J     bucle
fin:    HLT
