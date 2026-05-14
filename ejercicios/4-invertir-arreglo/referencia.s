; Invierte en su sitio las R1 palabras que empiezan en R6
        ADDI  R2, R6, 0         ; i = principio
        ADD   R3, R6, R1
        ADDI  R3, R3, -1        ; j = final
bucle:  SLT   R4, R2, R3        ; seguir mientras i < j
        LI    R5, 0
        BEQ   R4, R5, fin
        LW    R4, [R2+0]
        LW    R5, [R3+0]
        SW    R5, [R2+0]
        SW    R4, [R3+0]
        ADDI  R2, R2, 1
        ADDI  R3, R3, -1
        J     bucle
fin:    HLT
