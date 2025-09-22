; Suma 200 + 100 en 8 bits: el resultado es 44 con acarreo.
; Es la version en ensamblador de suma.hex.

        LDI A, 200
        LDI B, 100
        ADD A, B
        HLT
