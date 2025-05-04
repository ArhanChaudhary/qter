# TODO

- human-panic

## SCC

- Phase 2
  - pruning table
    - cartesian product tables
    - hash simd stuff
    - fix storage backend initialization meta
    - tANS table compression
    - cycle type pruning table
      - with fewer state spaces, go back to an exact pruning table
    - with fewer goal states, go back to an approximate table
    - each thread fills in 1024 entires at a time
    - exact: dfs at low levels instead of scanning
    - dynamic simd detection rather than -Ctarget-cpu=native
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
- Canonical ordering of stickers
- Define the moves as permutations and orientations of pieces
- Calculate orientations and parities of the puzzle
- Calculate the symmetries of the puzzle
- Parse our modified puzzlegeometry definition string
- Release as a crate on crates.io

## QAT

- Precompute tables for builtin architectures
- Refactor register references so that they assume the register declaration is global
- QAT Macros
  - Actual expansion
  - `after` syntax
  - Lua stuff
- Architecture switching
- Memory tapes
  - Implement in QAT
- Dynamically shuffle sub-cycles with syntax X ← A\*B\*C\*D, Y ← E\*F\*G\*H
- Function macro
- Directory of testing programs instead of hardcoding into Rust
  - Inline testing in the QAT format
- `solve-puzzle` and instruction to copy solving moves to other puzzle
- Analyzing branches and removing dead code
- Architecture that avoids sharing a piece by always having two additions simultaneously which avoids parity

## Interpreter/CLI

- Implement tapes
- Debugging tool
- Implementing the fancy CRT/loop-repetition-calculating thingy

## Q

- Compile to Q
- Parse Q
- "[repeat|print|halt] until _ solved" syntax
- optimize out immediate gotos after a label
- Asher's repeated move post process optimization: R U R repeated = R then U R2 repeated then R'
- force conditional blocks that end with "halt" to codegen at the end of the instruction memory, optimizing a goto

## End user

- Web app of qter with a visualization
- Youtube videos

## Robot

- Add robot to the README
