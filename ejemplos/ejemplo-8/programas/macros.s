; Macros e inclusiones: carga dos registros y los suma.
.incluir "constantes.inc"

.macro CARGA2 a, b, valor
    LDI \a, \valor
    LDI \b, \valor
.endm

        CARGA2 A, B, SIETE
        ADD A, B
        HLT
