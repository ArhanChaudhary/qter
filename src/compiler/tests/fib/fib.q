Puzzles
1: 3x3

    U L2 B' L U' B' U2 R B' R' B L
loop_1:
    print L' B' R B R' U2 B U L' B L2 U' until-solved UL UFL
continue_1:
    solved-goto UL UFL break_1
    D B2 D2 L' D' B2 D' B D2 L2 B' D F2 B2 U' F2 B2
    solved-goto LB DLB do_if_1
    goto after_if_1
do_if_1:
zero_loop_1:
    solved-goto UL UFL move_done_1
    D2 L D' F' D' R D' R U2 B R B2 U R' U F D
    goto zero_loop_1
move_done_1:
    halt "The number is: " U L D' U' F' L U F D F' L U' F2 L2 until-solved FR UFR
after_if_1:
    L2 F2 U L' F D' F' U' L' F U D L' U'
    goto continue_1
break_1:
    print D2 B2 L' D' R D' F L2 R U L' R2 until-solved LB DLB
continue_2:
    solved-goto LB DLB break_2
    R' D B' L2 U F2 R2 F2 U L F2 R2 D L' U2 L2
    solved-goto FR UFR do_if_2 HERE!
    goto after_if_2
do_if_2:
zero_loop_2:
    solved-goto LB DLB move_done_2
    F2 B2 U F2 B2 D' B L2 D2 B' D B2 D L D2 B2 D'
    goto zero_loop_2
move_done_2:
    halt "The number is: " L' B' R B R' U2 B U L' B L2 U' until-solved UL UFL
after_if_2:
    U L2 B' L U' B' U2 R B' R' B L
    goto continue_2
break_2:
    print U L D' U' F' L U F D F' L U' F2 L2 until-solved FR UFR
continue_3:
    solved-goto FR UFR break_3
    D' F' U' R U' B2 R' B' U2 R' D R' D F D L' D2
    solved-goto UL UFL do_if_3
    goto after_if_3
do_if_3:
zero_loop_3:
    solved-goto FR UFR move_done_3
    L2 U2 L D' R2 F2 L' U' F2 R2 F2 U' L2 B D' R
    goto zero_loop_3
move_done_3:
    halt "The number is: " D2 B2 L' D' R D' F L2 R U L' R2 until-solved LB DLB
after_if_3:
    R2 L U' R' L2 F' D R' D L B2 D2
    goto continue_3
break_3:
    goto loop_1
