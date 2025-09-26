; Copia la palabra guardada en `valor` a la RAM y se detiene.
; Muestra constantes, etiquetas hacia adelante, datos y varios segmentos.

DESTINO = 0x2000

        LD  A, [valor]
        ST  [DESTINO], A
        HLT

        .org 0x0100
valor:  .db 0x5A
mensaje: .db "Lorenz", 0
