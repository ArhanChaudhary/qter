<!-- cspell:disable -->

# TODO

⭐means it's important to be done before the video series
😎means it's optional

- crashlog

## SCC

- CCS
  - document solver.rs
    - update part of paper while at it
  - use enum dispatch for pruning table
  - check for logs in test cases; tracing
    - hardcode the first solution moves in the test cases to be sure
  - replace pub(crate) with getters
  - try out a different exact hasher
  - spam debug_assert!()
  - dont pack bit vector for AuxMem
  - solve for all cycle structures from CCF at once vs many runs of single cycle structure at a time
  - ⭐pruning table
    - generate table during the solve
      - if the number of probes exceeds the number of set values in the table by a certain factor (3x?) it considers generating another level of the table, if there is enough memory to do so
    - cartesian product tables
      - seed only one value
      - <https://discord.com/channels/772576325897945119/1326029986578038784/1347580846647017482>
    - fix storage backend initialization meta
    - tANS table compression
    - cycle type pruning table
      - with fewer state spaces, go back to an exact pruning table
      - generate the cycle type table before the approximate table and roughly guess the average pruning value
      - <https://discord.com/channels/772576325897945119/1326029986578038784/1374906236956573806>
    - ⭐each thread fills in 1024 entires at a time
    - ⭐exact: dfs at low levels instead of scanning
  - search
    - ⭐stabilizer
      - Look into fixing a corner for even cubes/other puzzles
    - ⭐standard symmetry
      - ⭐Doug's canon_into function simplified and explained in #programming
      - ⭐reread kociemba's website and h48.md
    - ⭐multithreading
      - heuristically sort based on sqrt(3)/sqrt(2) and canonical seq (h48.md)
      - microthreading
  - you NEED to account for parity constraints when calculating orbit size; address this in schreier sims
  - ⭐solved state for 4x4
  - ⭐antisymmetry
  - 😎mulcmp3 and mul3 optimizations from twsearch
  - 😎PGE
  - can we use move tables? look into at the end
  - Schreier Sims & generating algs using it
  - Generate a pruning table starting from the scramble instead of the solved state and then began the search from the solved state

## PuzzleGeometry

- ⭐Detect identical pieces
- ⭐Figure out algebraics
  - Acceptance criterion: All default puzzles can be processed in <1s with exact arithmetic
- ⭐Canonical ordering of stickers
- ⭐Calculate orientation and parity constraints of the puzzle
- ⭐Calculate the symmetries of the puzzle
- ⭐Parse our modified puzzlegeometry definition string
- ⭐Reorganize parts of qter_core into here, rename `puzzle_theory` or something, and release as a crate on crates.io
- Spherical cuts

## QAT

- ⭐Precompute tables for builtin architectures
- ⭐QAT Macros
  - ⭐Actual expansion
  - ⭐Lua stuff
- Architecture switching
- ⭐Memory tapes
  - ⭐Implement in QAT
- Dynamically shuffle sub-cycles with syntax X ← A\*B\*C\*D, Y ← E\*F\*G\*H
- Function macro
- ⭐Directory of testing programs instead of hardcoding into Rust
  - ⭐Inline testing in the QAT format
- 😎Instruction to copy solving moves to other puzzle
- 😎Architecture that avoids sharing a piece by always having two additions simultaneously which avoids parity
- 😎Asher's repeated move post process optimization: R U R repeated = R then U R2 repeated then R'
- 😎force conditional blocks that end with "halt" to codegen at the end of the instruction memory, optimizing a goto
- 😎Test with https://github.com/dtolnay/trybuild
- 😎Write a tree-sitter grammer for QAT

## Interpreter/CLI

- ⭐Implement tapes
- Debugging tool
- 😎Implementing the fancy CRT/loop-repetition-calculating thingy

## Q

- ⭐Compile to Q
  - ⭐"[repeat|print|halt] until _ solved" syntax
- Parse Q
- Comments with parentheses
- Write a tree-sitter grammer for Q

## End user

- Web app of qter with a visualization
- ⭐Youtube videos
- ⭐Animation of the robot doing a computation

## Robot

- ⭐Add robot to the README
- 😎Build one
