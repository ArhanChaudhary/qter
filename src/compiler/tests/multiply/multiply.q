Puzzles
A: 3x3

    input "Enter number X"
          L2 F2 U L' F D' F' U' L' F U D L' U'
          max-input 29
    solved-goto FR UFR l1
    input "Enter number Y"
          R2 L U' R' L2 F' D R' D L B2 D2
          max-input 29
    solved-goto UB DLB l2
l3:
    solved-goto UB l4
    F2 B2 U F2 B2 D' B L2 D2 B' D B2 D L D2 B2 D'
    solved-goto UB l5
    F2 B2 U F2 B2 D' B L2 D2 B' D B2 D L D2 B2 D'
    goto l3
l4:
    solved-goto ULF l6
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l4
l6:
    solved-goto UR ULF l7
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l6
l7:
    solved-goto UB DLB l8
    F' B' D2 R' B2 R U R2 B2 L' B' U B R2 L2 F R L'
    goto l7
l8:
    solved-goto ULF l9
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l8
l9:
    solved-goto UR ULF l10
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l9
l10:
    solved-goto FR UFR l11
    B2 L F D R2 F R' U F' R2 F D2 L2 D L B2
    goto l10
l11:
    solved-goto UR l12
    D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    goto l11
l12:
    solved-goto UR ULF l3
    F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l12
l13:
    solved-goto UB DLB l15
    U' R2 L2 U R2 F2 D2 R' F2 L' U2 L U L' B D' B
    goto l13
l15:
    solved-goto ULF l16
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l15
l16:
    solved-goto UR ULF l17
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l16
l17:
    solved-goto FR UFR l18
    U' R' L2 B' L' D' F' R F' D R' L B2 R2 L2 U' R2
    goto l17
l18:
    solved-goto UR l19
    D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    goto l18
l19:
    solved-goto UR ULF l20
    F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l19
l5:
    solved-goto ULF l21
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l5
l21:
    solved-goto UR ULF l20
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l21
l20:
    solved-goto DLB l13
    D2 B2 L' D' R D' F R L2 U R2 L'
    solved-goto UB DLB l42
    goto l49
l22:
    solved-goto UB l23
    D2 B2 L' D' R D' F R L2 U R2 L'
l49:
    U L2 B' L U' B' U2 R B' R' B L
    solved-goto UB l24
    F2 B2 U F2 B2 D' B L2 D2 B' D B2 D L D2 B2 D'
    solved-goto UB l24
    F2 B2 U F2 B2 D' B L2 D2 B' D B2 D L D2 B2 D'
    solved-goto UB l24
    F2 B2 U F2 B2 D' B L2 D2 B' D B2 D L D2 B2 D'
    solved-goto UB l24
    F2 B2 U F2 B2 D' B L2 D2 B' D B2 D L D2 B2 D'
    goto l22
l23:
    solved-goto ULF l25
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l23
l25:
    solved-goto UR ULF l26
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l25
l26:
    solved-goto UB DLB l27
    F2 B2 U F2 D L' B D2 B2 D L2 D' B R' L2 B2 R L
    goto l26
l27:
    solved-goto ULF l28
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l27
l28:
    solved-goto UR ULF l29
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l28
l29:
    solved-goto FR UFR l30
    D2 R' F' D2 R F R F' R' B' D F' L' D' B' L2 U' B2
    goto l29
l30:
    solved-goto UR l31
    D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    goto l30
l31:
    solved-goto UR ULF l22
    F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l31
l24:
    solved-goto ULF l32
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l24
l32:
    solved-goto UR ULF l33
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l32
l33:
    D2 B2 L' D' R D' F R L2 U R2 L'
    solved-goto UB l34
    R2 L U' R' L2 F' D R' D L B2 D2
l35:
    solved-goto UB DLB l36
    F' D' F' U' R B2 U2 D' R D F2 L B2 L D2 L2 D2
    goto l35
l36:
    solved-goto ULF l37
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l36
l37:
    solved-goto UR ULF l38
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l37
l38:
    solved-goto FR UFR l39
    D2 F U2 R' U D2 F D' R D R2 D F' R U R
    goto l38
l39:
    solved-goto UR l40
    D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    goto l39
l40:
    solved-goto UR ULF l33
    F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l40
l41:
    D2 B2 L' D' R D' F R L2 U R2 L'
l34:
    solved-goto UB DLB l42
    R2 L U' R' L2 F' D R' D L B2 D2
l43:
    solved-goto UB DLB l44
    R' B' R D F L2 U' B2 L2 B' U L2 U L' U' B2 L2 F'
    goto l43
l44:
    solved-goto ULF l45
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    goto l44
l45:
    solved-goto UR ULF l46
    D' R L' U' F' B2 L B U B L U R' D2 B' U'
    goto l45
l46:
    solved-goto FR UFR l47
    R' F2 D F' B2 L2 U L2 U F' B2 R D2 R' D' F2 D'
    goto l46
l47:
    solved-goto UR l48
    D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    goto l47
l48:
    solved-goto UR ULF l41
    F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l48
l1:
    solved-goto UB DLB l42
    D2 B2 L' D' R D' F R L2 U R2 L'
    goto l1
l2:
    solved-goto FR UFR l42
    U L U' D' F' L U F D F' L U' F2 L2
    goto l2
l42:
    halt "(X * Y) mod 30 ="
         U L U' D' F' L U F D F' L U' F2 L2
         counting-until FR UFR