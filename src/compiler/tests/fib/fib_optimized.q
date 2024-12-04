Puzzles
1: 3x3

    input "Which Fibonacci number to calculate: " B2 U2 L F' R B L2 D2 B R' F L
    solved-goto UFR label_1
    goto label_2
label_1:
    halt "The number is: 0"
label_2:
    D L' F L2 B L' F' L B' D' L'
label_3:
    L' F' R B' D2 L2 B' R' F L' U2 B2
    solved-goto UFR label_4
    goto label_5
label_4:
    halt "The number is: " L D B L' F L B' L2 F' L D' counting-until DL DFL
label_5:
    solved-goto DL DFL label_6
    L U' B R' L B' L' U' L U R2 B R2 D2 R2 D'
    goto label_5
label_6:
    L' F' R B' D2 L2 B' R' F L' U2 B2
    solved-goto UFR label_7
    goto label_8
label_7:
    halt "The number is: " F2 L2 U2 D' R U' B L' B L' U' counting-until FR DRF
label_8:
    solved-goto FR DRF label_9
    D' B' U2 B D' F' D L' D2 F' R' D2 F2 R F2 R2 U' R'
    goto label_8
label_9:
    L' F' R B' D2 L2 B' R' F L' U2 B2
    solved-goto UFR label_10
    goto label_11
label_10:
    halt "The number is: " U L' R' F' U' F' L' F2 L U R counting-until UF
label_11:
    solved-goto UF label_3
    B R2 D' R B D F2 U2 D' F' L2 F D2 F B2 D' L' U'
    goto label_11
