# TODO

## SCC

- Phase 2
  - pruning table
    - tANS table compression
    - cycle type pruning table
      - with fewer state spaces, go back to an exact pruning table
    - with fewer goal states, go back to an approximate table
  - stabilizer
  - avoid symmetric moves from the start
  - fix corner in stabilizer for 4x4
  - solved state for 4x4
  - standard symmetry
  - antisymmetry
  - fix sequence symmetry
  - multithreading
    - microthreading
  - make fsm lookup unsafe when pg is done
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

- Allow the robot guys' TUI to output the current cube state
- Fill in the `robot` command for our CLI that interacts with an external program that can interact with the robot
- Write an adapter program between the Qter CLI and the robot guys' TUI
