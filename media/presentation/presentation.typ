#import "@preview/polylux:0.4.0": *
#import "@preview/cetz:0.3.1"

/// The color palette of the theme
#let palette = (
  background: rgb("#2f3052"),///"#180b20"),
  text: rgb("#d8d9ff"),///white,
  primary: rgb("#424374"),///rgb("#40 choose 2823372"),
  accent: rgb("#8498f0"),///rgb("#df5671"),
)

/// My custom theme for Polylux. Designed for making coding related presentations. Requires the #link("https://github.com/githubnext/monaspace?tab=readme-ov-file#monaspace", [Monaspace]) font to be installed.
///
/// You can see what the theme looks like #link("https://github.com/Xendergo/wasm-session/blob/main/wasm.pdf", [here.])
///
///```typst
///#import code-theme: *
///
///#show: code-theme```
///
/// - aspect-ratio (string): The presentation's aspect ratio
#let code-theme(aspect-ratio: "16-9", body) = {
  set list(marker: ([#sym.bullet], [--]))

  set page(
    paper: "presentation-" + aspect-ratio,
    fill: palette.background,
    margin: 1em,
  )
  set text(fill: palette.text, size: 40pt, font: "Monaspace Neon Var")

  // set raw(theme: "Tomorrow-Night-Bright.tmTheme")

  show raw: it => block(
    // fill: rgb("#0f1419"),
    fill: palette.background,
    inset: 8pt,
    radius: 5pt,
    // width: 100%,
    // height: 100%,
    text(fill: palette.text, it, font: "Monaspace Neon Var")
  )

  body
}

#let polylux-slide = slide

/// A title slide
///
/// - title (content): The title of the presentation
/// - author (content): The author of the presentation
/// - note (content): A note at the bottom of the slide
#let title-slide(title: [], author: [], note: []) = {
  polylux-slide({
    set align(center + horizon)

    smallcaps(strong(title))
    block(line(start: (0pt, 0pt), length: 100%, stroke: palette.accent), width: 20%, above: 0.8em, below: 0.7em)
    text(size: .7em, author, font: "Monaspace Radon Var")

    place(top+left, polygon(fill: palette.primary, (0pt, 0pt), (50pt, 0pt), (0pt, 50pt)))
    place(bottom+right, polygon(fill: palette.primary, (50pt, 50pt), (50pt, 0pt), (0pt, 50pt)))

    align(bottom, block(text(size: .4em, note), width: 80%))
  })
}

/// A section slide
///
/// - title (content): The title of the section
/// - note (content): A note at the bottom of the slide
#let section-slide(title: [], note: []) = {
  polylux-slide({
    set align(center + horizon)

    smallcaps(strong(title))
    block(line(start: (0pt, 0pt), length: 100%, stroke: palette.accent), width: 20%, above: 0.8em, below: 0.3em)

    place(bottom+right, polygon(fill: palette.primary, (0pt, 0pt), (50pt, 0pt), (0pt, 50pt)))
    place(top+left, polygon(fill: palette.primary, (50pt, 50pt), (50pt, 0pt), (0pt, 50pt)))

    align(bottom, block(text(size: .4em, note), width: 80%))
  })
}

/// A slide split into left and right parts. The title and content are on the left side.
///
/// - title (content): The title of the slide
/// - content (content): The content of the slide
/// - picture (content): The content on the right part of the slide
/// - content-width (fraction): How much of the slide should be taken up by the content
#let split-slide(title: [], content: [], picture: [], content-width: .8fr) = {
  polylux-slide({
    grid(
      columns: (content-width, 1fr),
      column-gutter: .5em,
      box({
        place(top+right, line(start: (0pt, 0pt), length: 100%, angle: 90deg, stroke: palette.primary), dx: .5em)

        set text(size: 30pt)
        smallcaps(strong(title))
        block(line(start: (0pt, 0pt), length: 100%, stroke: palette.accent), width: 20%, above: 0.6em, below: 0.7em)
        set text(size: 20pt)
        content
      }, inset: .5em, width: 100%, height: 100%),
      box(align(center+horizon, box(align(left, picture))), width: 100%, height: 100%),
    )
  })
}

/// A slide with only a title and content
///
/// - title (content): The title of the slide
/// - content (content): The content of the slide
#let mono-slide(title: [], content: []) = {
  polylux-slide({
    set text(size: 30pt)
    smallcaps(strong(title))

    block(line(start: (0pt, 0pt), length: 100%, stroke: palette.accent), width: 20%, above: 0.6em, below: 0.8em)
    set text(size: 20pt)
    content
  })
}

/// A slide with only the content you put on it
///
/// - content (content): The content of the slide
#let empty-slide(content: []) = {
  polylux-slide({
    content
  })
}

// CODE STARTS HERE

#let colors = (
  "r": rgb("#d86f9a"),///red,
  "o": rgb("#e4b37f"),///orange,
  "w": rgb("#d8d9ff"),///white,
  "y": rgb("#e1e485"),///yellow,
  "b": rgb("#8498f0"),///blue,
  "g": rgb("#2cda9d"),///green,
  "n": rgb("#424374"),///gray,
)

#let cube(faces, offset: (0, 0), scale-amt: 1) = {
  import cetz.draw : *

  let faces = faces.split(" ").map(v => v.split("").filter(n => colors.keys().contains(n)).map(n => colors.at(n)))

  set-style(stroke: palette.background)

  group({
    translate(offset)
    scale(scale-amt)

    let ortho-squish = 1 / (2*calc.cos(30deg))

    let transforms = (
      {
        scale(x: 1, y: ortho-squish)
        rotate(90deg + 45deg)
      },
      {
        translate((-2.12, -1.22))
        rotate(120deg)
        scale(x: 1, y: ortho-squish)
        rotate(45deg)
      },
      {
        rotate(60deg)
        scale(x: 1, y: ortho-squish)
        rotate(135deg)
      }
    );

    for (facelets, transform) in faces.zip(transforms) {
      group({
        transform

        for i in range(0, 3) {
          for j in range(0, 3) {
            rect((i + 0.03, j + 0.03), (i + 0.97, j + 0.97), radius: .2, fill: facelets.at(2 - i + j * 3))
          }
        }
      })
  }
  })
}

#show: code-theme

#title-slide(title: [QTER], author: [Arhan, Henry, Asher])

#section-slide(
  title: [What is qter?],
)

#empty-slide(
  content: [
    #align(center, [
      #text(size: 15pt)[```
      Puzzles
      A: 3x3

      1  | input "First number"
                 R' F' L U' L U L F U' R
                 max-input 90
      2  | input "Second number"
                 U F R' D' R2 F R' U' D
                 max-input 90
      3  | B2 R L2 D L' F' D2 F' L2
           B' U' R D' L' B2 R F
      4  | solved-goto DFR FR 6
      5  | goto 3
      6  | R' F' L U' L U L F U' R
      7  | R' U F' L' U' L' U L' F R
      8  | solved-goto ULF UL 13
      9  | R' U F' L' U' L' U L' F R
      10 | solved-goto ULF UL 13
      11 | U F R' D' R2 F R' U' D
      12 | goto 7
      13 | halt "The average is"
                D' U R F' R2 D R F' U'
                counting-until DFR FR

      ```]
    ])
  ]
)

#section-slide(
  title: [How does qter work?],
)

#mono-slide(
  title: [How does qter work?],
  content: [
    #align(right + horizon, alternatives-fn(start: 0, count: 6, slide => {
      let state = (
        "rrrrrrrrr wwwwwwwww ggggggggg",
        "rrrrrrrrr gggwwwwww yyygggggg",
        "rrrrrrrrr yyywwwwww bbbgggggg",
        "rrrrrrrrr bbbwwwwww wwwgggggg",
      ).at(calc.rem(slide - 1, 4))

      let alg = (
        $"\"Zero\""$,
        $"\"One\""$,
        $"\"Two\""$,
        $"\"Three\""$,
        $"\"Four?\""$,
      ).at(slide - 1)

      cetz.canvas({
        import cetz.draw : *

        cube(state, scale-amt: 1.5)
        content((-10, 1), text(size: 30pt)[$alg$], anchor: "west")
        circle((7, 0), radius: 0)
      })
    }))
])

#mono-slide(
  title: [Can we do math?],
  content: [
    #only(1)[$1 + 2 → ("Up") ("Up" "Up")$]
    #only(2)[$1 + 2 → ("Up") ("Up" "Up") = "Up" "Up" "Up" → 3$]
    #align(right + horizon, alternatives-fn(start: 0, count: 3, slide => {
      let state = (
        "rrrrrrrrr gggwwwwww yyygggggg",
        "rrrrrrrrr bbbwwwwww wwwgggggg",
      ).at(calc.rem(slide - 1, 4))

      let alg = (
        $"\"One\""$,
        $"\"Three\""$,
      ).at(slide - 1)

      cetz.canvas({
        import cetz.draw : *

        cube(state, scale-amt: 1.5)
        content((-10, 1), text(size: 30pt)[$alg$], anchor: "west")
        circle((7, 0), radius: 0)
      })
    }))
  ]
)

#mono-slide(title: [Bigger numbers?], content: [
  #align(right + horizon, cetz.canvas({
    import cetz.draw : *

    content((-12, 1), $("Right") ("Up")$, anchor: "west")
    cube("rrrrrrwww gggwwowwo ryygggggg", scale-amt: 1.5)
    circle((7, 0), radius: 0)
  }))
])

#mono-slide(title: [Conditional jump?], content: [
  #align(right + horizon, cetz.canvas({
    import cetz.draw : *

    content((-12, 1), $(("Right") ("Up")) × ?$, anchor: "west")
    cube("oybrroyrr oywwwrwwr ggwbggywb", scale-amt: 1.5)
    circle((7, 0), radius: 0)
  }))
])

#mono-slide(title: [Conditional jump?], content: [
  #align(right + horizon, cetz.canvas({
    import cetz.draw : *

    content((-12, 1), $(("Right") ("Up")) × 0 $, anchor: "west")
    cube("rrrrrrrrr wwwwwwwww ggggggggg", scale-amt: 1.5)
    circle((7, 0), radius: 0)
  }))
])

#mono-slide(title: [Multiple registers?], content: [
  #align(right + horizon, cetz.canvas({
    import cetz.draw : *

    content((-12, 1), $("Up") ("Down")$, anchor: "west")
    cube("rrrrrrrrr gggwwwggg yyygggyyy", scale-amt: 1.5)
    circle((7, 0), radius: 0)
  }))
])

#mono-slide(title: [Examples of architectures], content: [
  = 1260
  - R U2 D' B D'

  = 90×90
  - R' F' L U' L U L F U' R
  - U F R' D' R2 F R' U' D

  = 30×30×30
  - U L2 B' L U' B' U2 R B' R' B L
  - R2 L U' R' L2 F' D R' D L B2 D2
  - L2 F2 U L' F D' F' U' L' F U D L' U'
])

#mono-slide(title: [Examples of architectures], content: [
  = 30×18×10×9
  - U L B' L B' U R' D U2 L2 F2
  - D L' F L2 B L' F' L B' D' L'
  - R' U' L' F2 L F U F R L U'
  - B2 U2 L F' R B L2 D2 B R' F L
])

#mono-slide(title: [What about solved-goto?], content: [
  #align(right + horizon, cetz.canvas({
    import cetz.draw : *

    content((-14, 1), $"Register \"Up\" is zero"$, anchor: "west")
    cube("rrrrrrrrr wwwwwwggg ggggggyyy", scale-amt: 1.5)
    circle((7, 0), radius: 0)
  }))
])

#mono-slide(title: [What about solved-goto?], content: [
  #align(center + horizon, cetz.canvas({
    import cetz.draw : *

    content((1.6, 5), [solved-goto UF UFR 8])

    content((-5.3, 3), $"Branch taken"$, anchor: "west")
    cube("nnnnrnnrr nwwnwnnnn gnnngnnnn", scale-amt: 1.5)

    content((-5.3 + 9.3, 3), $"Branch not taken"$, anchor: "west")
    cube("nnnnrnnrw nwgnwnnnn rnnngnnnn", scale-amt: 1.5, offset: (10, 0))
  }))
])

#mono-slide(title: [How does this look in Q?], content: [
  ```
  Puzzles
  A: 3x3
  B: 3x3

  1 | U D
  2 | goto 1
  3 | solved-goto UF UFR 2
  4 | switch B
  ```
])

#section-slide(
  title: [QAT]
)

#mono-slide(
  title: [Register declaration],
  content: [
    #one-by-one[][
      ```janet
      .registers {
        A, B ← 3x3 builtin (90, 90)
      }
      ```
    ][
      ---
      ```janet
      .registers {
        A ← 3x3 builtin (1260)
        B ← 3x3 builtin (1260)
      }
      ```
    ]
  ]
)

#mono-slide(
  title: [Primitive instructions],
  content: [
    #alternatives-fn(position: top + left, start: 1, count: 7, slide => {
      let instructions = "
        add A 1
        spot:
        goto spot
        solved-goto A spot
        input \"Your favorite number:\" A
        halt \"The result is\" A
      ".split("\n").map(v => v.trim(" ")).filter(v => v != "")

      let v = instructions.slice(0, count: slide - 1).join("\n")

      raw(if v == none { "" } else { v }, lang: "janet")
    })
  ]
)

#split-slide(
  title: [Macros],
  content: [
    #uncover("2-")[```janet
    .macro if {
      (solved $R:reg $code:block) => {
            solved-goto $R do_if
            goto after_if
        do_if:
            $code
        after_if:
      }
    }
    ```]
    #uncover("3-4")[
      ```janet
      if solved A {
        add A 5
      }
      ```
    ]
  ],
  picture: [
    #uncover(4)[
      #set text(20pt)
      ```janet
          solved-goto A do_if
          goto after_if
      do_if:
          add A 5
      after_if:
      ```
    ]
  ],
  content-width: 1.2fr,
)

#section-slide(
  title: [Turing completeness],
)

#mono-slide(
  title: [How can qter be turing complete?],
  content: [
    #align(center + horizon, image("Turing_Machine_Model_Davey_2012.jpg", height: 80%))
  ]
)

#split-slide(
  title: [How can qter be turing complete?],
  content: [
    ```
    Puzzles
    tape A: 3x3

    1 | move-right A
    2 | switch-tape A
    3 | R U
    4 | move-left A
    ```
  ],
  picture: [
    #alternatives-fn(start: 0, count: 6, i => {
      let i = i - 1

      cetz.canvas({
        import cetz.draw : *

        content((-2, 2), [#set text(15pt); Next instruction: #i])

        cube("rrrrrrrrr wwwwwwwww ggggggggg", offset: (0, -3))
        if i > 0 {
          cube(if i <= 2 { "rrrrrrrrr wwwwwwwww ggggggggg" } else { "rrrrrrwww gggwwowwo ryygggggg" }, offset: (6, -3))
        }

        let base-x = -2.12

        let holding-pos = (0, 0, 1, 1, 1).at(i) * 6

        content((base-x + holding-pos, 0), [#set text(15pt); #align(center, [Holding])])
        line((base-x + holding-pos, -0.4), (base-x + holding-pos, -1.4), stroke: white, mark: (end: "straight"))

        let head-pos = (0, 1, 1, 1, 0).at(i) * 6

        content((base-x + head-pos, -8.5), [#set text(15pt); #align(center, [Head])])
        line((base-x + head-pos, -8.5 + 0.4), (base-x + head-pos, -8.5 + 1.4), stroke: white, mark: (end: "straight"))
      })
    })
  ]
)

#section-slide(
  title: [But wouldn't qter just be brainfuck?]
)

#mono-slide(
  title: [But wouldn't qter just be brainfuck?],
  content: [
    - Multiple tapes are allowed
    #show: later
    - This makes call stacks easy
    #show: later
    - We can use a global register to keep track of the head position
  ]
)

#mono-slide(
  title: [But wouldn't qter just be brainfuck?],
  content: [
    ```janet
    .macro index {
      ($tape:tape $current:reg $to:reg) => {
        while not-solved $current {
          dec $current
          move-left $tape
        }

        while not-solved $to {
          dec $to
          inc $current
          move-right $tape
        }
      }
    }
    ```
  ]
)

#section-slide(
    title: [How do we find qter registers?],
    note: [An extremely simplified overview]
)

#mono-slide(
  title: [Qter Architecture Solver],
  content: [
    - Computes optimal qter registers in two phases
      - Cycle Combination Prover: Find best cycles that provably exist
      - Cycle Combination Solver: Find shortest algorithms that produce the cycles
])

#mono-slide(
  title: [Cycle Combination Prover],
  content: [
    The maximum number of repetitions for an algorithm on the Rubik's cube is 1260
    #show: later
    #v(30pt)
    This is formed from:
    - LCM 56 on edges: 4 cycle, another 4 cycle, and 7 cycle
    - LCM 45 on corners: 9 cycle and 15 cycle
    - LCM(45, 56) = 1260
])

#mono-slide(
  title: [Cycle Combination Prover],
  content: [
    - We can generalize this idea!
    #show: later
      - N registers, not just one
    #show: later
      - Any twisty puzzle, like the 4x4x4 or megaminx
])

#mono-slide(
  title: [Cycle Combination Solver],
  content: [
    We have a structure of the cycles we want. Now, find an actual algorithm for the cycle.
    #show: later
    - The algorithm must be as short as possible
    #show: later
    - The only known optimal solving technique is brute force :-(
  ]
)

#mono-slide(
  title: [Cycle Combination Solver],
  content: [
    #let node(faces) = {
       cetz.canvas({
         cube(faces, scale-amt: 0.5)
       })
    }
    #align(center, cetz.canvas({
      import cetz.draw : *


      stroke(5pt + white)
      let data = (
        [#node("rrrrrrrrr wwwwwwwww ggggggggg")],
        ([#text(size: 33pt)[\...]],),
        ([#node("rrrrrrrrr gggwwwwww yyygggggg")],),
        ([#node("rrrrrrrrr yyywwwwww bbbgggggg")],),
        (
          [#node("rrrrrrrrr bbbwwwwww wwwgggggg")],
          [
            #text(size: 33pt)[\...]
            #h(10pt)
            $node("rrbrrwrrw bbowwowwo ggwggwggw")$
            #h(10pt)
            $node("rrorrorro bbywwywwg ggggggwww")$
            #h(10pt)
            $node("rrrrrrwgg bwwbwwbww owwoggogg")$
            #h(10pt)
            #text(size: 33pt)[\...]
          ],
        ),
        ([#node("rrwrrwrrw wwowwowwo ggggggggg")],),
        ([#text(size: 33pt)[\...]],),
      )
      cetz.tree.tree(
        data,
        spread: 3, grow: 4.5,
        direction: "down",
        draw-node: (node, ..) => {
          content((), [#node.content])
        },
        draw-edge: (from, to, ..) => {
          let (a, b) = (from + ".center", to + ".center")
          line((a, 1.3, b), (b, 1.5, a));
        },
        name: "tree"
      )

      line((to: "tree.g0-3", rel: (-0.9, -1)), (to: "tree.0-3-0", rel: (-4.5, 1.2)))
      line((to: "tree.g0-3", rel: (-0.6, -1.15)), (to: "tree.0-3-0", rel: (-2, 1.2)))
      line((to: "tree.g0-3", rel: (0.6, -1.15)), (to: "tree.0-3-0", rel: (2, 1.2)))
      line((to: "tree.g0-3", rel: (0.9, -1)), (to: "tree.0-3-0", rel: (4.5, 1.2)))
    }))
])

#mono-slide(
  title: [Cycle Combination Solver],
  content: [
    Modified Korf's algorithm
    #show: later
    - Iterative DFS + heuristic
    #show: later
    - Movecount coefficient calculator
    #show: later
    - Fixed pieces
    #show: later
    - \... The optimizations gets complicated
  ]
)

#mono-slide(
  title: [We integrated Qter into a robot!],
  content: [
    Raw video: https://drive.google.com/file/d/121oxXZX2t8l1pAY0NNbxVoiUOWuV8dqR/view?usp=drive_link

    Slo-mo video: https://drive.google.com/file/d/1dQrUkTKFgRiQjZEsESq42mu1uAC41Vrr/view?usp=drive_link

  ]
)

#mono-slide(
  title: [We demoed Qter at OpenSauce 2025!],
  content: [
    #grid(
      columns: (1fr, 1fr),
      image("robot.JPG", height: 84%),
      image("us.jpg"),
    )
  ]
)

#section-slide(
  title: [The future of qter]
)

#section-slide(
  title: [Thank you!],
  note: [Any questions?],
)