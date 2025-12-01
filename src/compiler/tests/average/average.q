Puzzles
A: 3x3

1 | input "First number"
          R' F' L U' L U L F U' R
          max-input 90
2 | input "Second number"
          U F R' D' R2 F R' U' D
          max-input 90
3 | solved-goto DFR FR 7
4 | B2 R L2 D L' F' D2 F' L2
    B' U' R D' L' B2 R F
5 | solved-goto ULF UL 15
6 | goto 3
7 | R' F' L U' L U L F U' R
8 | R' U F' L' U' L' U L' F R
9 | solved-goto ULF UL 14
10 | R' U F' L' U' L' U L' F R
11 | solved-goto ULF UL 14
12 | U F R' D' R2 F R' U' D
13 | goto 8
14 | halt until DFR FR solved
          "The average is"
          D' U R F' R2 D R F' U'

15 | repeat until DFR FR
         B2 R L2 D L' F' D2 F' L2
         B' U' R D' L' B2 R F
16 | R' F' L U' L U L F U' R
17 | R' U F' L' U' L' U L' F R
18 | solved-goto ULF UL 22
19 | R' U F' L' U' L' U L' F R
20 | solved-goto ULF UL 22
21 | U F R' D' R2 F R' U' D
22 | goto 17
23 | halt until DFR FR solved
          "The average is"
          D' U R F' R2 D R F' U'
