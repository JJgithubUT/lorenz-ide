; Suma los numeros del 1 al 10 y guarda el resultado en la RAM.
;   R1 = contador, R2 = acumulador, R3 = limite, R4 = direccion del resultado

        LI    R1, 1
        LI    R2, 0
        LI    R3, 11
bucle:  ADD   R2, R2, R1
        ADDI  R1, R1, 1
        BNE   R1, R3, bucle
        LI    R4, 0x100
        ADD   R4, R4, R4          ; 0x200
        ADD   R4, R4, R4          ; 0x400
        ADD   R4, R4, R4          ; 0x800
        ADD   R4, R4, R4          ; 0x1000
        ADD   R4, R4, R4          ; 0x2000
        ADD   R4, R4, R4          ; 0x4000: primera direccion de la RAM
        SW    R2, [R4+0]
        HLT
