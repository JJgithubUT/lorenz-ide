; R1 = mem[R6] + mem[R6 + 1]
        LW    R2, [R6+0]
        LW    R3, [R6+1]
        ADD   R1, R2, R3
        HLT
