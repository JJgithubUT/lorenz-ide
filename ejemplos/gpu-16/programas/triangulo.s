; Dibuja un triangulo azul: la CPU escribe cada palabra del comando en el puerto 0x20 (comando).
        LI    R1, 1          ; operacion COLOR
        OUT   0x20, R1
        LI    R1, 0x1F       ; azul en RGB565
        OUT   0x20, R1
        LI    R1, 5          ; operacion TRIANGULO
        OUT   0x20, R1
        LI    R1, 2
        OUT   0x20, R1       ; x0
        OUT   0x20, R1       ; y0
        LI    R2, 20
        OUT   0x20, R2       ; x1
        OUT   0x20, R1       ; y1
        OUT   0x20, R1       ; x2
        OUT   0x20, R2       ; y2
        HLT
