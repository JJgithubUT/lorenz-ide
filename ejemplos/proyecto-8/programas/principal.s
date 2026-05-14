; Carga una constante definida en otro archivo y la suma a si misma.
        .externo valor
        .org 0x10
        LDI A, valor
        LDI B, valor
        ADD A, B
        HLT
