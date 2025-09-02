<!-- cspell:disable -->

# TODO

⭐means it's important to be done before the video series
😎means it's optional

- crashlog

## SCC

- Phase 2
  - check for logs in test cases; tracing
    - hardcode the first solution moves in the test cases to be sure
  - make sure sequence symmetry is good
  - replace pub(crate) with getters
  - try out a different exact hasher
  - spam debug_assert!()
  - dont pack bit vector for AuxMem
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
    - dynamic simd detection rather than -Ctarget-cpu=native
  - ⭐stabilizer
  - ⭐avoid symmetric moves from the start by doing A\* then IDA\* in parallel (youtube video)
  - you NEED to account for parity constraints when calculating orbit size; address this in schreier sims
  - fix corner in stabilizer for 4x4
  - ⭐solved state for 4x4
  - ⭐standard symmetry
    - ⭐reread kociemba's website and h48.md
  - ⭐antisymmetry
  - ⭐multithreading
    - microthreading
  - ⭐make fsm lookup unsafe when pg is done
  - can we use move tables? look into at the end
- ⭐Phase 1
- Look into fixing a corner for even cubes/other puzzles
- Schreier Sims & generating algs using it

## PuzzleGeometry

- ⭐Canonical ordering of stickers
- ⭐Output ksolve stuff
- ⭐Calculate orientations and parities of the puzzle
- ⭐Calculate the symmetries of the puzzle
- ⭐Parse our modified puzzlegeometry definition string
- ⭐Reorganize parts of qter_core into here, rename `puzzle_theory` or something, and release as a crate on crates.io

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

## Interpreter/CLI

- ⭐Implement tapes
- Debugging tool
- 😎Implementing the fancy CRT/loop-repetition-calculating thingy

## Q

- ⭐Compile to Q
  - ⭐"[repeat|print|halt] until _ solved" syntax
- Parse Q
- Comments with parentheses

## End user

- Web app of qter with a visualization
- ⭐Youtube videos
- ⭐Animation of the robot doing a computation

## Robot

- ⭐Add robot to the README
- 😎Build one
