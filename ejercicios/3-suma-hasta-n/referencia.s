; R2 = 1 + 2 + ... + R1
        LI    R2, 0             ; acumulador
        LI    R3, 1             ; contador
        ADDI  R4, R1, 1         ; el bucle termina cuando el contador llega a N + 1
bucle:  BEQ   R3, R4, fin
        ADD   R2, R2, R3
        ADDI  R3, R3, 1
        J     bucle
fin:    HLT
