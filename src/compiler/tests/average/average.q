Puzzles
A: 3x3

1  | input "First number"
           R' F' L U' L U L F U' R
           max-input 90
2  | input "Second number"
           U F R' D' R2 F R' U' D
           max-input 90
3  | B2 R L2 D L' F' D2 F' L2
     B' U' R D' L' B2 R F
4  | solved-goto DFR FR 6
5  | goto 3
6  | R' F' L U' L U L F U' R
7  | R' U F' L' U' L' U L' F R
8  | solved-goto ULF UL 13
9  | R' U F' L' U' L' U L' F R
10 | solved-goto ULF UL 13
11 | U F R' D' R2 F R' U' D
12 | goto 7
13 | halt "The average is"
          D' U R F' R2 D R F' U'
          counting-until DFR FR
