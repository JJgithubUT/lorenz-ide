; R2 = fibonacci(R1)
        LI    R2, 0             ; a
        LI    R3, 1             ; b
        LI    R5, 0
bucle:  BEQ   R1, R5, fin
        ADD   R4, R2, R3        ; a + b
        ADDI  R2, R3, 0         ; a = b
        ADDI  R3, R4, 0         ; b = a + b
        ADDI  R1, R1, -1
        J     bucle
fin:    HLT
