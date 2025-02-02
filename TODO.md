# TODO

⭐for high priority items (ideally before we meet up with the robot guys)

## SCC

- Phase 2
  - standard symmetry
  - inverse symmetry
  - canonical sequences
    - commutative ordering FSM
      - prevent same face
      - prevent antipodes
    - sequence symmetry
  - multithreading
  - find all solutions at depth
  - tANS table compression
- Phase 1
- Look into fixing a corner for even cubes/other puzzles

## QAT

- ⭐Precompute tables for builtin architectures
- ⭐Optimize additions using the precomputed tables
- ⭐`A%2` syntax
- Refactor register references so that they assume the register declaration is global
- QAT Macros
  - Actual expansion
  - `after` syntax
  - Lua stuff
- Memory tapes
  - Implement in QAT
- Dynamically shuffle sub-cycles with syntax X ← A\*B\*C\*D, Y ← E\*F\*G\*H
- Function macro
- Directory of testing programs instead of hardcoding into Rust
  - Inline testing in the QAT format
<!-- Aren't these two are the same thing -->
- Translate multiplication program to QAT
- Henry's efficient multiplication program

## Interpreter/CLI

- Implement tapes
- ⭐Dumping an execution trace
- Debugging tool

## Q

- Compile to Q
- Parse Q

## End user

- Web app of qter with a visualization
- Youtube videos

## Robot

- ⭐Find calibration algs
- Computer vision algorithm
- The webapp and rust program that will run the Q program
