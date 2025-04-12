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
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
    repeat until UB DLB solved
           F' B' D2 R' B2 R U R2 B2 L' B' U B R2 L2 F R L'
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
    repeat until FR UFR solved
           B2 L F D R2 F R' U F' R2 F D2 L2 D L B2
    repeat until UR solved
           D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    repeat until UR ULF solved
           F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l3
l13:
    repeat until UB DLB solved
           U' R2 L2 U R2 F2 D2 R' F2 L' U2 L U L' B D' B
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
    repeat until FR UFR solved
           U' R' L2 B' L' D' F' R F' D R' L B2 R2 L2 U' R2
    repeat until UR solved
           D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    repeat until UR ULF solved
           F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l20
l5:
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
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
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
    repeat until UB DLB solved
           F2 B2 U F2 D L' B D2 B2 D L2 D' B R' L2 B2 R L
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
    repeat until FR UFR solved
           D2 R' F' D2 R F R F' R' B' D F' L' D' B' L2 U' B2
    repeat until UR solved
           D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    repeat until UR ULF solved
           F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l22
l24:
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
l33:
    D2 B2 L' D' R D' F R L2 U R2 L'
    solved-goto UB l34
    R2 L U' R' L2 F' D R' D L B2 D2
l35:
    repeat until UB DLB solved
           F' D' F' U' R B2 U2 D' R D F2 L B2 L D2 L2 D2
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
    repeat until FR UFR solved
           D2 F U2 R' U D2 F D' R D R2 D F' R U R
    repeat until UR solved
           D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    repeat until UR ULF solved
           F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l33
l41:
    D2 B2 L' D' R D' F R L2 U R2 L'
l34:
    solved-goto UB DLB l42
    R2 L U' R' L2 F' D R' D L B2 D2
l43:
    repeat until UB DLB solved
           R' B' R D F L2 U' B2 L2 B' U L2 U L' U' B2 L2 F'
    repeat until ULF solved
           D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    repeat until UR ULF solved
           D' R L' U' F' B2 L B U B L U R' D2 B' U'
    repeat until FR UFR solved
           R' F2 D F' B2 L2 U L2 U F' B2 R D2 R' D' F2 D'
    repeat until UR solved
           D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    repeat until UR ULF solved
           F2 L' B' D F2 U' R2 F U2 R' D' B U2 F' L2 U
    goto l41
l1:
    repeat until UB DLB solved
           D2 B2 L' D' R D' F R L2 U R2 L'
    goto l42
l2:
    repeat until FR UFR solved
           U L U' D' F' L U F D F' L U' F2 L2
l42:
    halt "(X * Y) mod 30 ="
         U L U' D' F' L U F D F' L U' F2 L2
         counting-until FR UFR