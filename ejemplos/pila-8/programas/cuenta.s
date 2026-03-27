; Suma 3 cinco veces: el contador esta en 0x4000 y el acumulador en 0x4001.

        PUSH  5
        STORE 0x4000
        PUSH  0
        STORE 0x4001
bucle:  LOAD  0x4001
        PUSH  3
        ADD
        STORE 0x4001
        LOAD  0x4000
        PUSH  1
        SUB
        DUP
        STORE 0x4000
        JZ    fin
        JMP   bucle
fin:    HLT
