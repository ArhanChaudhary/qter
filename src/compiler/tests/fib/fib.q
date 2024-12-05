Puzzles
A: 3x3

1  | input "Which Fibonacci number to calculate: "
           B2 U2 L F' R B L2 D2 B R' F L
           max-repeat 8
2  | solved-goto UFR 4
3  | goto 5
4  | halt "The number is: 0"
5  | D L' F L2 B L' F' L B' D' L'
6  | L' F' R B' D2 L2 B' R' F L' U2 B2
7  | solved-goto UFR 9
8  | goto 10
9  | halt "The number is: "
          L D B L' F L B' L2 F' L D' counting-until DL DFL
10 | solved-goto DL DFL 13
11 | L U' B R' L B' L' U' L U R2 B R2 D2 R2 D'
12 | goto 10
13 | L' F' R B' D2 L2 B' R' F L' U2 B2
14 | solved-goto UFR 16
15 | goto 17
16 | halt "The number is: "
          F2 L2 U2 D' R U' B L' B L' U' counting-until FR DRF
17 | solved-goto FR DRF 20
18 | D' B' U2 B D' F' D L' D2 F' R' D2 F2 R F2 R2 U' R'
19 | goto 17
20 | L' F' R B' D2 L2 B' R' F L' U2 B2
21 | solved-goto UFR 23
22 | goto 24
23 | halt "The number is: "
          U L' R' F' U' F' L' F2 L U R counting-until UF
24 | solved-goto UF 6
25 | B R2 D' R B D F2 U2 D' F' L2 F D2 F B2 D' L' U'
26 | goto 24
