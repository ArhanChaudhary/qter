Puzzles
A: 3x3

    input "Enter number X" C
    solved-goto FR UFR l1
    input "Enter number Y" B
    solved-goto UB DLB l2
l3:
    solved-goto UB l4
    [1, 29, 0]
    solved-goto UB l5
    [1, 29, 0]
    goto l3
l4:
    solved-goto ULF l6
    [29, 1, 0]
    goto l4
l6:
    solved-goto UR ULF l7
    [27, 3, 0]
    goto l6
l7:
    solved-goto UB DLB l8
    [1, 28, 0]
    goto l7
l8:
    solved-goto ULF l9
    [29, 1, 0]
    goto l8
l9:
    solved-goto UR ULF l10
    [27, 3, 0]
    goto l9
l10:
    solved-goto FR UFR l11
    [2, 0, 29]
    goto l10
l11:
    solved-goto UR l12
    [29, 0, 1]
    goto l11
l12:
    solved-goto UR ULF l3
    [20, 0, 10]
    goto l12
l13:
    solved-goto UR ULF l14
    [29, 1, 0]
    goto l13
l14:
    solved-goto UB DLB l15
    [1, 27, 0]
    goto l14
l15:
    solved-goto ULF l16
    [29, 1, 0]
    goto l15
l16:
    solved-goto UR ULF l17
    [27, 3, 0]
    goto l16
l17:
    solved-goto FR UFR l18
    [3, 0, 29]
    goto l17
l18:
    solved-goto UR l19
    [29, 0, 1]
    goto l18
l19:
    solved-goto UR ULF l20
    [20, 0, 10]
    goto l19
l5:
    solved-goto ULF l21
    [29, 1, 0]
    goto l5
l21:
    solved-goto UR ULF l20
    [27, 3, 0]
    goto l21
l20:
    solved-goto DLB l13
l22:
    solved-goto UB l23
    [1, 29, 0]
    solved-goto UB l24
    [1, 29, 0]
    solved-goto UB l24
    [1, 29, 0]
    solved-goto UB l24
    [1, 29, 0]
    solved-goto UB l24
    [1, 29, 0]
    goto l22
l23:
    solved-goto ULF l25
    [29, 1, 0]
    goto l23
l25:
    solved-goto UR ULF l26
    [27, 3, 0]
    goto l25
l26:
    solved-goto UB DLB l27
    [1, 25, 0]
    goto l26
l27:
    solved-goto ULF l28
    [29, 1, 0]
    goto l27
l28:
    solved-goto UR ULF l29
    [27, 3, 0]
    goto l28
l29:
    solved-goto FR UFR l30
    [5, 0, 29]
    goto l29
l30:
    solved-goto UR l31
    [29, 0, 1]
    goto l30
l31:
    solved-goto UR ULF l22
    [20, 0, 10]
    goto l31
l24:
    solved-goto ULF l32
    [29, 1, 0]
    goto l24
l32:
    solved-goto UR ULF l33
    [27, 3, 0]
    goto l32
l33:
    [0, 29, 0]
    solved-goto UB l34
    [0, 1, 0]
l35:
    solved-goto UB DLB l36
    [1, 23, 0]
    goto l35
l36:
    solved-goto ULF l37
    [29, 1, 0]
    goto l36
l37:
    solved-goto UR ULF l38
    [27, 3, 0]
    goto l37
l38:
    solved-goto FR UFR l39
    [7, 0, 29]
    goto l38
l39:
    solved-goto UR l40
    [29, 0, 1]
    goto l39
l40:
    solved-goto UR ULF l33
    [20, 0, 10]
    goto l40
l41:
    [0, 29, 0]
l34:
    solved-goto UB DLB l42
    [0, 1, 0]
l43:
    solved-goto UB DLB l44
    [1, 19, 0]
    goto l43
l44:
    solved-goto ULF l45
    [29, 1, 0]
    goto l44
l45:
    solved-goto UR ULF l46
    [27, 3, 0]
    goto l45
l46:
    solved-goto FR UFR l47
    [11, 0, 29]
    goto l46
l47:
    solved-goto UR l48
    [29, 0, 1]
    goto l47
l48:
    solved-goto UR ULF l41
    [20, 0, 10]
    goto l48
l1:
    solved-goto UB DLB l42
    [0, 29, 0]
    goto l1
l2:
    solved-goto FR UFR l42
    [0, 0, 29]
    goto l2
l42:
    halt "(X * Y) mod 30 =" C
