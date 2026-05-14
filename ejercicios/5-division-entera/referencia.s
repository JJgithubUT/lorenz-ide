; R3 = R1 / R2, R4 = R1 % R2
        LI    R3, 0
        ADDI  R4, R1, 0         ; el resto empieza siendo el dividendo
        LI    R6, 0
bucle:  SLT   R5, R4, R2        ; resto < divisor ?
        BNE   R5, R6, fin
        SUB   R4, R4, R2
        ADDI  R3, R3, 1
        J     bucle
fin:    HLT
