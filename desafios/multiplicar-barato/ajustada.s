; R0 vale 0 al arrancar: sirve de cero sin gastar una carga.
        LI    R3, 0
bucle:  BEQ   R2, R0, fin
        ADD   R3, R3, R1
        ADDI  R2, R2, -1
        J     bucle
fin:    HLT
