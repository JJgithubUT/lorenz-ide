; Escribe 5 y 7 en el periferico externo y lee el total (12) en B.
        LDI   A, 5
        OUT   A
        LDI   A, 7
        OUT   A
        IN    B
        HLT
