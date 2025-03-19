# TODO

## SCC

- Phase 2
  - pruning table
    - tANS table compression
  - stabilizer
  - standard symmetry
  - antisymmetry
  - fix sequence symmetry
  - multithreading
    - microthreading
- Phase 1
- Look into fixing a corner for even cubes/other puzzles
- Schreier Sims & generating algs using it

## PuzzleGeometry

- Get a permutation group out of a puzzle definition
- Define the moves as permutations and orientations of pieces
- Calculate orientations and parities of the puzzle
- Calculate the symmetries of the puzzle
- Parse our modified puzzlegeometry definition string
- Release as a crate on crates.io

## QAT

- Precompute tables for builtin architectures
- `A%2` syntax
- Refactor register references so that they assume the register declaration is global
- QAT Macros
  - Actual expansion
  - `after` syntax
  - Lua stuff
- Architecture switching
- Memory tapes
  - Implement in QAT
- Asher's repeated move post process optimization: R U R repeated = R then U R2 repeated
- Dynamically shuffle sub-cycles with syntax X ← A\*B\*C\*D, Y ← E\*F\*G\*H
- Function macro
- Directory of testing programs instead of hardcoding into Rust
  - Inline testing in the QAT format
- Henry's efficient multiplication program
- `solve-puzzle` and `repeat-until` optimizations
- Analyzing branches and removing dead code

## Interpreter/CLI

- Implement tapes
- Debugging tool
- Implementing the fancy CRT/loop-repetition-calculating thingy

## Q

- Compile to Q
- Parse Q
- "[repeat|print|halt] until _ solved" syntax

## End user

- Web app of qter with a visualization
- Youtube videos

## Robot

- Find calibration algorithms
- Computer vision algorithm
- The webapp and rust program that will run the Q program
