<!-- cspell:disable -->

# TODO

⭐means it's important to be done before the video series

- crashlog

## SCC

- Phase 2
  - replace pub(crate) with getters
  - SortedCycleType creation error
  - ⭐branding for OrbitPuzzleStates should be unique and happen at a very small level
  - ⭐pruning table
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
    - dynamic simd detection rather than -Ctarget-cpu=native
  - ⭐stabilizer
  - ⭐avoid symmetric moves from the start
  - you NEED to account for parity constraints when calculating orbit size; address this in schreier sims
  - fix corner in stabilizer for 4x4
  - ⭐solved state for 4x4
  - ⭐standard symmetry
    - ⭐reread kociemba's website and h48.md
  - ⭐antisymmetry
  - ⭐multithreading
    - microthreading
  - ⭐make fsm lookup unsafe when pg is done
- ⭐Phase 1
- Look into fixing a corner for even cubes/other puzzles
- Schreier Sims & generating algs using it

## PuzzleGeometry

- ⭐Canonical ordering of stickers
- ⭐Output ksolve stuff
- ⭐Calculate orientations and parities of the puzzle
- ⭐Calculate the symmetries of the puzzle
- ⭐Parse our modified puzzlegeometry definition string
- Use quadratic numbers instead of floats
- Reorganize parts of qter_core into here, rename `puzzle_theory` or something, and release as a crate on crates.io

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
- `solve-puzzle` and instruction to copy solving moves to other puzzle
- Architecture that avoids sharing a piece by always having two additions simultaneously which avoids parity

## Interpreter/CLI

- ⭐Implement tapes
- Debugging tool
- Implementing the fancy CRT/loop-repetition-calculating thingy

## Q

- ⭐Compile to Q
- ⭐"[repeat|print|halt] until _ solved" syntax
- Parse Q
- Comments with parentheses
- Asher's repeated move post process optimization: R U R repeated = R then U R2 repeated then R'
- force conditional blocks that end with "halt" to codegen at the end of the instruction memory, optimizing a goto

## End user

- Web app of qter with a visualization
- ⭐Youtube videos

## Robot

- ⭐Add robot to the README
- Build one
