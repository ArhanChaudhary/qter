<!-- cspell:ignore nlogo promela scilab vcube benwh rokicki lgarron ditrus voltara infinidoge esqu1 trangium movecount Arhan Chaudhary Rovnyak korf twizzle metaprogramming cubies youtuber abelian nxopt sebastianotronto twsearch -->
<p align="center">
    <img src="media/Header.png" width="500" alt="The qter logo">
</p>
<hr>

PROJECT STATUS: almost MVP; many features incomplete

No part of this README was written with AI.

Qter is a human-friendly Rubik's cube computer. This means you can compile a program and physically execute it by turning a Rubik's cube (or any twisty puzzle), without knowing how computers work. Following is an example executable program that accepts an index as user input and computes the corresponding Fibonacci number. It is written in our custom Rubik's cube file format named Q:

`fib.q`
<!-- some alternatives: clarity cl el janet lfe lean nlogo opa pact promela scilab -->
```l
Puzzles
A: 3x3

1 | input "Which Fibonacci number to calculate:"
           B2 U2 L F' R B L2 D2 B R' F L
           max-input 8
2 | solved-goto UFR 14
3 | D L' F L2 B L' F' L B' D' L'
4 | L' F' R B' D2 L2 B' R' F L' U2 B2
5 | solved-goto UFR 15
6 | repeat until DL DFL solved
            L U' B R' L B' L' U'
            L U R2 B R2 D2 R2 D'
7 | L' F' R B' D2 L2 B' R' F L' U2 B2
8 | solved-goto UFR 16
9 | repeat until FR DRF solved
            D' B' U2 B D' F' D L' D2
            F' R' D2 F2 R F2 R2 U' R'
10 | L' F' R B' D2 L2 B' R' F L' U2 B2
11 | solved-goto UFR 17
12 | repeat until UF solved
            B R2 D' R B D F2 U2 D'
            F' L2 F D2 F B2 D' L' U'
13 | goto 4
14 | halt "The number is: 0"
15 | halt until DL DFL solved
         "The number is"
         L D B L' F L B' L2 F' L D'
16 | halt until FR DRF solved
          "The number is"
          F2 L2 U2 D' R U' B L' B L' U'
17 | halt until UF solved
          "The number is"
          U L' R' F' U' F' L' F2 L U R
```

This was compiled from our custom high level programming language named QAT (Qter Assembly Text):

`fib.qat`

```janet
.registers {
    A, B, C, D <- 3x3 builtin (30, 18, 10, 9)
}

.macro fib-shuffle {
    ($R1:reg $R2:reg $R3:reg $counter:reg) => {
        dec $counter
        if solved $counter {
            halt "The number is" $R1
        }
        while not-solved $R1 {
            dec $R1
            inc $R2
            inc $R3
        }
    }
}

input "Which Fibonacci number to calculate:" D
if solved D {
    halt "The number is: 0"
}
inc B
loop {
    fib-shuffle B A C D
    fib-shuffle A C B D
    fib-shuffle C B A D
}
```

### Table of Contents

- [How does qter work?](#how-does-qter-work)
  - [Introduction](#introduction)
  - [Addition](#addition)
  - [Bigger numbers](#bigger-numbers)
  - [Branching](#branching)
  - [Multiple numbers](#multiple-numbers)
- [Physically running qter](#physically-running-qter)
  - [The Q file format](#the-q-file-format)
  - [Logical instructions](#logical-instructions)
  - [Other twisty puzzles](#other-twisty-puzzles)
- [The QAT programming language](#the-qat-programming-language)
  - [Your first QAT program](#your-first-qat-program)
  - [Global variables](#global-variables)
  - [Basic instructions](#basic-instructions)
  - [Metaprogramming](#metaprogramming)
  - [Prelude](#prelude)
- [Memory tapes](#memory-tapes)
- [Technical details](#technical-details)
  - [Cycle combination solver](#cycle-combination-solver)
    - [Phase 1](#phase-1)
    - [Phase 2](#phase-2)
  - [Puzzle geometry](#puzzle-geometry)
- [About the authors](#about-the-authors)
- [References](#references)
- [Acknowledgements](#acknowledgements)

# How does qter work?

## Introduction

First, we will assume that you are familiar with [standard move notation](https://jperm.net/3x3/moves) for a Rubik's cube.

The most important thing for a computer to be able to do is represent numbers. Let's take a solved cube and call it "zero":

<img src="media/State Zero.png" width="275" alt="A solved Rubik's cube"/>

The fundamental unit of computation in qter is an _algorithm_, or a sequence of moves to apply to the cube. Let's see what happens if we apply the simplest algorithm, just turning the top face, and see what this buys us:

<img src="media/State One.png" width="275" alt="A Rubik's cube after performing U"/>

Now, let's call this state "one". Since applying the algorithm `U` transitioned the cube from state "zero" to state "one", perhaps applying `U` _again_ could transition us from state "one" to state "two":

<img src="media/State Two.png" width="275" alt="A Rubik's cube after performing U U"/>

And again to state "three":

<img src="media/State Three.png" width="275" alt="A Rubik's cube after performing U U U"/>

And... again to state "four"?

<img src="media/State Four.png" width="275" alt="A Rubik's cube after performing U U U U"/>

If we apply the algorithm `U` four times, we find that it returns back to state "zero". This means that we can't represent every possible number with this scheme. We should have expected that, because the Rubik's cube has a _finite_ number of states whereas there are an _infinite_ amount of numbers.

This doesn't mean that we can't do math though, we just have to treat numbers as if they "wrap around" at four. This is analogous to the way analog clocks wrap around after twelve. The difference between our scheme and analog clocks is that we will consider the solved state to represent "zero" instead of "four".

## Addition

Can we justify this way of representing numbers? Let's consider adding "two" to "one". We reach the "two" state using the algorithm `U U`, so if we apply that algorithm to the "one" state, we will find the cube in the same state as if we applied `(U) (U U)`, or `U U U`, which is exactly how we reach the state labelled "three". It's easy to see that associativity of moves makes addition valid in this scheme. What if we wanted to add "three" to "two"? We would expect a result of "five", but since the numbers wrap around upon reaching four, we would actually expect to reach the state of "one". You can try on your own Rubik's cube and see that it works.

What if we want to perform subtraction? We know that addition is performed using an algorithm, so can we find an algorithm that adds a negative number? Let's consider the state that represents "one". If we subtract one, we would expect the cube to return to state "zero". The algorithm that brings the cube from state "one" to state "zero" is `U'`. This is exactly the inverse of our initial `U` algorithm. If we want to subtract two, we can simply subtract one twice as before: `U' U'`.

You may notice that subtracting one is equivalent to adding three, because `U'` is equivalent to `U U U`. It may seem like this impedes our ability to do math, but by working through an example one can see that it doesn't: Adding three to one gives four, but since four wraps around to zero, our result is actually zero, as if we subtracted one. In general, any number may be seen as positive or negative: -1 = 3, -2 = 2, and -3 = 1. You can manually verify this yourself if you like.

## Bigger numbers

If the biggest number qter could represent was three, it would not be an effective tool for computation. Thankfully, the Rubik's cube has 43 quintillion states, leaving us lots of room to do better than just four. Consider the algorithm `R U`. We can play the same game using this algorithm. The solved cube represents zero, `R U` represents one, `R U R U` represents two, etc. This algorithm performs a much more complicated action on the cube, so we should be able to represent more numbers. In fact, the maximum number we can represent this way is 104, wrapping around after 105 iterations. We would say that the algorithm has "order 105".

There are still lots of cube states left; can we do better? Unfortunately, it's only possible to get to 1259, wrapping around on the 1260th iteration. You can try this using the algorithm `R U2 D' B D'`. This has been proven to be the maximum order [[1](#ref-1)].

## Branching

The next thing that a computer must be able to do is _branch_, without it we can only do addition and subtraction and nothing else. If we want to perform loops or only execute code conditionally, qter must be able to change what it does based on the state of the cube. For this, we introduce a "solved-goto" instruction.

If you perform `R U` on a cube a bunch of times without counting, it's essentially impossible for you to tell how many times you did the algorithm by _just looking_ at the cube. With one exception: If you did it _zero_ times, then the cube is solved and it's completely obvious that you did it zero times. Since we want qter code to be executable by humans, the "solved-goto" instruction asks you to go to a different location of the program _only_ if the cube is solved.

## Multiple numbers

If you think about what programs you could actually execute with just a single number and a "jump if zero" instruction, it would be almost nothing. What would be wonderful is if we could represent _multiple_ numbers on the cube at the same time.

If you're familiar with how to solve a cube layer by layer, you should know that it's possible for an algorithm to affect only a small section of the cube at once, for example just the last layer. Therefore, it should be possible to represent two numbers using two algorithms that affect distinct "areas" of the cube.

The simplest example of this concept are the algorithms `U` and `D`. You can see that `U` and `D` both allow representing numbers up to three, and since they affect different areas of the cube, we can represent _two different_ numbers on the cube at the _same time_. We call these "registers", as an analogy to the concept in classical computing.

As described, "solved-goto" would only branch if the entire cube is solved, however since each algorithm affects a distinct area of the cube, it's possible for a human to determine whether a _single_ register is zero, by inspecting whether a particular section of the cube is solved. In the Q format, we support this by each "solved-goto" instruction giving a list of positions, all of which must have their solved piece for the branch to be taken, but not necessarily any more.

Can we do better than two registers with four states? In fact we can! If you try out the algorithms `R' F' L U' L U L F U' R` and `U F R' D' R2 F R' U' D`, you can see that they affect different pieces and both have order ninety. You may notice that they both rotate the same corner; this is not a problem because they are independently decodable even ignoring that corner. One of the biggest challenges in the development of qter has been finding sets of algorithms with high orders that are all independently decodable.

If you're curious, the Fibonacci program uses these four registers:

| Order | Algorithm |
| ----- | --------- |
| 30 | `U L B' L B' U R' D U2 L2 F2` |
| 18 | `D L' F L2 B L' F' L B' D' L'` |
| 10 | `R' U' L' F2 L F U F R L U'` |
| 9 | `B2 U2 L F' R B L2 D2 B R' F L` |

# Physically running qter

## The Q file format

The Q file format is qter's representation of a computer program in an executable Rubik's cube language. The file format was designed in such a way that, with only basic Rubik's cube knowledge, a human can physically manipulate a twisty puzzle to execute a program and perform a meaningful computation.

Qter doesn't just support 3x3x3 cubes, but it works with any twisty puzzle in the shape of a platonic solid. Since most people are most familiar with the 3x3x3 cube, we will introduce qter with the aforementioned from now on.

Q files are expected to be read from top to bottom. Each line indicates an instruction, the simplest of which is just an algorithm to perform on the cube. For example:

```l
Puzzles
A: 3x3

1 | U' R2
2 | L D'
...
```

The `Puzzles` declaration specifies the types of twisty puzzles used. In this example, it is declaring that you must start with a 3x3x3 cube, and that it has the name "A". The name is unimportant in this example, but becomes important when operating on multiple cubes. The instructions indicate that you must perform the algorithm `U' R2 L D'` on the Rubik's cube, given in [standard move notation](https://jperm.net/3x3/moves). You must begin with the cube solved before following the instructions.

The Q file format also includes special instructions that involve the twisty puzzle but require additional logic. These logical instructions are designed to be simple enough for humans to understand and perform.

## Logical instructions

Following this section, you should be able to understand how to physically execute the example Fibonacci program provided at the beginning of this document. More complicated instructions are expanded upon in the next section.

- `goto <number>`

<ul>

Jump to the specified line number instead of reading on to the next line. For example:

```l
Puzzles
A: 3x3

1 | U' R2
2 | L D'
3 | goto 1
...
```

Indicates an infinite loop of performing `U' R2 L D'` on the Rubik's cube. After performing the algorithm, the `goto` instruction requires you to jump back to line 1 where you started.

</ul>

- `solved-goto <number> <positions>`

<ul>

If the specified positions on the puzzle each contain their solved piece, then jump to the line number specified as if it was a `goto` instruction. Otherwise, fall through and go to the next instruction. For example:

```l
Puzzles
A: 3x3

1 | U' R2
2 | solved-goto 4 UFR UF
3 | goto 1
4 | L D'
...
```

Indicates repeatedly performing `U' R2` until the UFR corner position and UB edge position contain their solved pieces. Then, perform `L D'` on the Rubik's cube. Note that three faces uniquely identify any corner position and two faces uniquely identify any edge position on the Rubik's cube, hence UFR (up front right) and UF (up front).

Determining if a position contains its solved piece slightly varies from puzzle to puzzle, but the idea remains the same. For the Rubik's cube, this is the case when each face of the piece at the position is the same color as its center.

The following illustrates a successful `solved-goto 4 UFR UF` instruction where you would jump to line 4:

<img src="media/solved-goto-success-dark.png" width="200" alt="A Rubik's cube with the UFR and UF positions solved">

The following illustrates an unsuccessful `solved-goto 4 UFR UF` instruction where you would skip it and continue to the next instruction. Note that _all_ pieces specified need to be in their solved positions, not just one:

<img src="media/solved-goto-failure-dark.png" width="200" alt="A Rubik's cube with the UFR and UF positions unsolved">

For other twisty puzzles, see [Other twisty puzzles](#other-twisty-puzzles).

</ul>

- `solve`

<ul>

Solve the puzzle using your favorite method. Logically, this instruction zeroes out all registers on the puzzle.

</ul>

- `repeat until <positions> solved <algorithm>`

<ul>

Repeat the given algorithm until the given positions contain their solved pieces. Logically, this is equivalent to

```l
N   | solved-goto N+3 <positions>
N+1 | <algorithm>
N+2 | goto N
N+3 | ...
```

but is easier to read and understand. This pattern occurs enough in Q programs that it is worth defining an instruction for it.

</ul>

- `input <message> <algorithm> max-input <number>`

<ul>

This instruction facilitates arbitrary input from a user which will be stored and processed on the puzzle. To do so, repeat the given algorithm "your input" number of times. For example:

```l
Puzzles
A: 3x3

1 | input "Choose 0-5"
          R U R' U'
          max-input 5
...
```

To input the number two, execute the algorithm `(R U R' U') (R U R' U')` on the Rubik's cube. Notice that if you try to execute `R U R' U'` six times, the cube will return to its solved state as if you had inputted the number zero. Thus, your input number must not be greater than five, and this is shown with the `max-input 5` syntax.

If a negative input is meaningful to the program you are executing, you can input negative one by performing the inverse of the algorithm. For example, negative two would be inputted as `(U R U' R') (U R U' R')`.

</ul>

- `halt <message> [<algorithm> counting-until <positions>]`

<ul>

This instruction terminates the program and gives an output, and it is similar to the `input` instruction. To decode the output of the program, repeat the given algorithm until the positions given are solved (see the `solved-goto` instruction). The number of repetitions it took to solve the given pieces along with the specified message is considered the output of the program. For example:

```l
Puzzles
A: 3x3

1 | input "Choose 0-5"
          R U R' U'
          max-input 5
2 | halt "You chose"
          U R U' R'
          counting-until UFR
```

In this example, after performing the input and reaching the halt instruction, you would have to repeat `U R U' R'` until the UFR corner is solved. For example, if you inputted the number two by performing `(R U R' U') (R U R' U')`, the expected output will be two, since you have to perform `U R U' R'` twice to solve the UFR corner. Therefore, the expected output of the program is "You chose 2".

If the program does not require giving a numeric output, then the algorithm may be left out. For example:

```l
Puzzles
A: 3x3

1 | halt "I halt immediately"
```

</ul>

- `print <message> [<algorithm> counting-until <positions>]`

<ul>

This is an optional instruction that you may choose to ignore. The `print` instruction serves as a secondary mechanism to produce output without exiting the program. The motivation stems from the fact that, without this instruction, the only form of meaningful output is the single number produced by the `halt` instruction.

To execute this instruction, repeat the given algorithm until the positions are solved, analogous to the halt instruction. The number of repetitions this took is then the output of the print statement. Then, you must perform the inverse of the algorithm the same number of times, undoing what you just did and returning the puzzle to the state it was in before executing the print instruction. For example:

```l
Puzzles
A: 3x3

1 | R U R2 B2 U L U' L' D' R' D R B2 U2
2 | print "This should output ten:"
          R U counting-until UFR UF
3 | halt "This should also output ten:"
          R U counting-until UFR UF
```

Like the `halt` instruction, including just a message is legal. In this case, you can skip this instruction as there is nothing to do. For example:

```l
Puzzles
A: 3x3

1 | print "Just a friendly debugging message :-)"
...
```

</ul>

- `switch <letter>`

<ul>

Put down your current puzzle and pick up a different one, labeled by letter in the `Puzzles` declaration. It is important that you do not rotate the puzzle when setting it aside or picking it back up. For example:

```l
Puzzles
A: 3x3
B: 3x3

1 | U
2 | switch B
3 | R
...
```

This program requires two Rubik's cubes to execute. The instructions indicate performing `U` on the first Rubik's cube and then `R` on the second. When the program starts, you are expected to be holding the first cube in the list.

</ul>

## Other twisty puzzles

WIP

Talking points:

- The `Puzzles` declaration accepts a hard-coded puzzle name or a PuzzleGeometry description
- PuzzleGeometry is a format developed and designed by Tomas Rokicki that generates a puzzle definition from a simple description. You can read more about it [here](https://alpha.twizzle.net/explore/help.html#puzzlegeometry) and are encouraged to experiment with the format interactively on [Twizzle Explorer](https://alpha.twizzle.net/explore) (click "Config").

# The QAT programming language

This section assumes moderate familiarity with an existing programming language, such as Python, JavaScript, or C. Once again, qter supports all types of twisty puzzles in the shape of a platonic solid. Since most people are most familiar with the 3x3x3 cube, we will introduce qter with the aforementioned from now on.

## Your first QAT program

If you have experience working with a compiled programming language, you know that to run a program, you compile your source code into machine code that the computer processor then interprets and executes. The qter compilation pipeline works similarly.

<p align="center">
  <img src="media/CompilationPipelineVert.svg" width="360" alt="A diagram of the qter compilation pipeline">
</p>

Qter's high level programming language is called QAT, or Qter Assembly Text. To run your first QAT program, you will first need to install Cargo (talk about installing Cargo) and then the qter compiler executable through the command line: `cargo install qter`. Once set up, create a file named `average.qat` with the following program code.

```janet
.registers {
    A, B <- 3x3 builtin (90, 90)
}

    -- Calculate the average of two numbers
    input "First number:" A
    input "Second number:" B
    print "Calculating average..."
sum_loop:
    add A 1
    add B 89
    solved-goto B found_sum
    goto sum_loop
found_sum:
    add A 1
divide_by_2:
    add A 89
    solved-goto A stop
    add A 89
    solved-goto A stop
    add B 1
    goto divide_by_2
stop:
    halt "The average is" B
```

To compile this program, run `qter compile average.qat` to generate `average.q`. To execute it, run `qter interpret average.q` and enter your favorite two numbers into the prompts.

## Global variables

Every QAT program begins with a `.registers` statement, used to declare global variables named registers. The statement in the above average program declares two global registers of size 90 to be stored on a Rubik's cube. That is, additions operate modulo 90: incrementing a register of value 89 resets it back to 0, and decrementing a register of value 0 sets it to 89.

The `builtin` keyword refers to the fact that valid register sizes are specified in a puzzle-specific preset. For the Rubik's cube, all builtin register sizes are in [src/qter_core/puzzles/3x3.txt](src/qter_core/puzzles/3x3.txt). Unlike traditional computers, qter is only able to operate with small and irregular register sizes.

You can choose to use larger register sizes at the cost of requiring more puzzles. For example, 1260 is a valid builtin register size that needs an entire Rubik's cube to declare. If your program wants access to more than one register, it would have to use multiple Rubik's cubes for more memory.

```janet
.registers {
    A <- 3x3 builtin (1260)
    B <- 3x3 builtin (1260)
    ...
}
```

The `.registers` statement is also used to declare memory tapes, which help facilitate local variables, call stacks, and heap memory. This idea will be expanded upon in [Memory tapes](#memory-tapes).

## Basic instructions

The basic instructions of the QAT programming language mimic an assembly-like language. If you have already read [The Q file format](#the-q-file-format), notice the similarities with QAT.

- `add <variable> <number>`

<ul>

Add a constant number to a variable. This is the only way to change the value of a variable.

</ul>

- `goto <label>`

<ul>

Jump to a label, an identifier used to mark a specific location within code. The syntax for declaring a label follows the common convention amongst assembly languages:

```janet
infinite_loop:
    goto infinite_loop
```

</ul>

- `solved-goto <variable> <label>`

<ul>

Jump to a label if the specified variable is zero. The name of this instruction is significant in the Q file format.

</ul>

- `input <message> <variable>`

<ul>

Ask the user for numeric input, which will be added to the given variable.

</ul>

- `print <message> [<variable>]`

<ul>

Output a message, optionally followed by a variable's value.

</ul>

- `halt <message> [<variable>]`

<ul>

Terminate the program with a message, optionally followed by a variable's value.

</ul>

## Metaprogramming

WIP

Talking points:

- macros
- .define
- lua

## Prelude

WIP

Talking points:

- convenience macros like `inc`, `dec`, and control flow
- [Link to prelude](src/qter_core/prelude.qat) and encourage its reference

# Memory tapes

Henry you could write the rest of this section if you want

- `move-left <tape> <number>`

<ul>

WIP

</ul>

- `move-right <tape> <number>`

<ul>

WIP

</ul>

- `switch-tape <tape>`

<ul>

WIP

</ul>

# Technical details

## Cycle combination solver

(This section is a huge WIP)

Qter's cycle combination solver computes the optimal computer architecture for a puzzle using any amount of cycles.

On the Rubik's Cube, every type of piece can only move to positions occupied by the same type of piece. These sets of disjoint positions are called _orbits_. The Rubik's cube has three types of pieces: edges, corners, and centers. Corner pieces can only move to corner positions and edge pieces can only move to edge positions. Disallowing rotations, center pieces are always in their solved positions because they cannot move. An important implication of this fact is that colors must be positioned around their centers to solve the Rubik's cube.

So, the Rubik's cube has one orbit of twelve edge pieces and another orbit of eight corner pieces. We will refer to pieces as _cubies_ from now on.

A Rubik's cube _move_ is defined by a manipulation of a single face, defined by the half-turn metric. That is, a 180 degree rotation of a face is considered a single move. To express moves, we use the [standard move notation](https://jperm.net/3x3/moves). An _algorithm_ is a sequence of moves. For example the algorithm `F2 U'` indicates turning the front face 180 degrees followed by turning the top face 90 degrees counterclockwise.

If you repeat an algorithm enough times, you will always be brought back where you started. You may have already tried this yourself from the solved state: if you keep repeating an algorithm such as `R U` you will eventually re-solve the cube. The number of repetitions this takes is the algorithm's _order_, and the set of positions visited by repeating the algorithm forms its _cycle_. In this example, since `R U` has to be repeated 105 times to be brought back to its original state, its order is 105.

The proof for this introduces an important concept that will later be brought up again, [group theory](https://en.wikipedia.org/wiki/Group_theory). The set of moves on the Rubik's cube forms an algebraic structure called a _group_. Since there are only finitely many states a Rubik's cube can be in, this group is finite. It is an early theorem of group theory that every element (algorithm) of a finite group has finite order [[2](#ref-2)].

At its most basic level, _registers_ are units of storage that can be modified at will, and an essential building block to how computers work. Computer CPUs use registers to store small amounts of data during program execution.

Armed with puzzle theory background knowledge, you can understand the main idea of how qter works in three words: cycles are registers. In essence, the value of a register is the Nth step of the cycle's corresponding algorithm.

Let's explain what that means through an example. Here are all the states visited by the `U` cycle:

<p align="center">
  <img src="media/U States.png" width="415" alt="The states visited by U"/>
</p>

The `U` cycle takes four repetitions to re-solve the cube, therefore it has order four. We claim that this is structurally identical to a two bit register (a register of size four).

- We have a way to increment a register by a constant
- 1260 order is maximal [[1](#ref-1)] and not enough for any meaningful computation
- Generalize the notion of a register to multiple cycles that coexist on the Rubik's cube
- The set of pieces affected by a given cycle must not interfere with the set of pieces affected by any other cycle. Helpful to think no longer in terms of moves but in cycles as in these pieces may be moved but they are restored. If the pieces were to interfere, then this would mean modifying the value of one register has a side effect of modifying the value of another unintended register.
- More registers mean more states 90*90 > 1260

### Phase 1

WIP

Talking points

- Based on [[1](#ref-1)]
- Find all ways to assign cubies to orbits, then find the max order using partitions and priority queue
- Pareto front

### Phase 2

WIP

Using Korf's algorithm

Talking points

- Korf's algorithm
- Symmetry and inverse reduction [[3](#ref-3)]
- Trangium's algorithm evaluator

Using GAP

WIP

Talking points

- Stabilizers
- Conjugacy classes
- Fork of optimal solver

## Puzzle geometry

# About the authors

- Arhan Chaudhary: Hi! I am a sophomore at Purdue University, and I have always been fascinated by Rubik's cubes since I learned how to solve them in middle school. I was greatly inspired by the [Purdue Hackers](https://www.purduehackers.com/) community to begin this project, and have spent the better part of the entire school year working on it. I'm looking for Summer 2026 internships - and I'm particularly excited about working with startups. Read more about my work at my [website](https://arhan.sh/).
- Henry Rovnyak: (_lands behind you_) Hello there! Like Arhan, I too am a sophomore at Purdue University. I'm interested in math and programming, and I met Arhan through this project and the Purdue Hackers community. I enjoy systems and scientific programming, but I also have a soft spot for theoretical work and frontend design. Arhan may or may not have gotten me addicted to cubes... I'm also interested in Summer 2026 internships, and you should consider checking out [my website](https://hrovnyak.gitlab.io/) to see some of the other stuff I've been working on.
- Asher Gray: Hello! I'm a data analyst and youtuber from the PNW. I started off making videos about solving Rubik's cubes blindfolded, and now focus on math and fun ways to visualize it. Lately I've been studying the theory of abelian groups, including the abelian subgroups of the Rubik's cube. I'm excited to have joined this project, such an interesting application of these ideas! You can find me on [YouTube](https://m.youtube.com/channel/UCJZt93WO-evfsfi7YlVmrQA) or enjoy some interactive math visualizations on my [website](https://thegraycuber.github.io/).

# References
<!-- cspell:disable -->
[<span id="ref-1">1</span>] Bergvall, O., Hynning, E., Hedberg, M., Mickelin, J., & Masawe, P. (2010). On Rubik’s cube. _Report, KTH Royal Institute of Technology_ (pp. 65-79). \
[<span id="ref-2">2</span>] Chris Grossack (<https://math.stackexchange.com/users/655547/chris-grossack>), Prove cycling in a Rubik's cube, URL (version: 2019-12-11): <https://math.stackexchange.com/q/3472935> \
[<span id="ref-3">3</span>] Rokicki, T., Kociemba, H., Davidson, M., & Dethridge, J. (2014). The diameter of the rubik's cube group is twenty. _siam REVIEW, 56_(4), 645-670.
<!-- cspell:enable -->
# Acknowledgements

Qter's theoretical background is built upon the hard work of:

- [@rokicki](https://github.com/rokicki)'s [nxopt](https://github.com/rokicki/cube20src/blob/master/nxopt.md), [twsearch](https://github.com/cubing/twsearch), and [PuzzleGeometry](https://alpha.twizzle.net/explore/help.html#puzzlegeometry).
- [@lgarron](https://github.com/lgarron)'s [canonical FSM](https://github.com/cubing/twsearch/blob/42d41c3480e79dc50005346f0ca7c4727530237f/src/rs/_internal/canonical_fsm/canonical_fsm.rs).
- [@ScriptRacoon](https://github.com/ScriptRacoon)'s [order solver](https://gist.github.com/ScriptRaccoon/c12c4884c116dead62a15a3d09732d5d).
- [@trangium](https://github.com/trangium)'s [Movecount Coefficient Calculator](https://trangium.github.io/MovecountCoefficient/).
- [@sebastianotronto](https://github.com/sebastianotronto)'s [multithreaded IDA* implementation](https://git.tronto.net/h48/file/src/solvers/h48/solve.h.html).
- [@Voltara](https://github.com/Voltara)'s [SIMD optimizations](https://github.com/Voltara/vcube/blob/9f5bc2cce18f29437879ace825f22917f6705378/src/avx2_cube.h).

Additionally,

- Thanks to [@rokicki](https://github.com/rokicki) for personally advising most of phase 2's design.
- Thanks to [@benwh1](https://github.com/benwh1) for miscellaneous puzzle theory insights.
- Thanks to [@Infinidoge](https://github.com/Infinidoge) for providing access to powerful hardware.
- Thanks to [@DitrusNight](https://github.com/DitrusNight) for advising qter's programming language design.
- Thanks to [@Phineas1500](https://github.com/Phineas1500) for encouragement and outreach.
