use crate::PuzzleDescriptionString;

enum PuzzleBaseShape {
    Tetrahedron,
    Cube,
    Octahedron,
    Dodecahedron,
    Icosahedron,
}

enum PuzzleCutType {
    Face,
    Vertex,
    Edge,
}

struct PuzzleCutDescription {
    cut_type: PuzzleCutType,
    distance: f64,
}

pub struct PuzzleDescription {
    shape: PuzzleBaseShape,
    cuts: Vec<PuzzleCutDescription>,
}

impl PuzzleDescription {
    pub fn from(puzzle_description_string: PuzzleDescriptionString) -> Result<Self, &'static str> {
        let args = puzzle_description_string
            .split_whitespace()
            .collect::<Vec<_>>();
        if args.len() % 2 == 0 {
            return Err("Invalid puzzle description argument count");
        }
        let shape = args[0];
        let shape = match shape {
            "o" => PuzzleBaseShape::Octahedron,
            "c" => PuzzleBaseShape::Cube,
            "i" => PuzzleBaseShape::Icosahedron,
            "d" => PuzzleBaseShape::Dodecahedron,
            "t" => PuzzleBaseShape::Tetrahedron,
            _ => return Err("Invalid puzzle description shape"),
        };
        let cuts = args[1..]
            .chunks(2)
            .map(|chunk| {
                let cut_type = match chunk[0] {
                    "f" => PuzzleCutType::Face,
                    "v" => PuzzleCutType::Vertex,
                    "e" => PuzzleCutType::Edge,
                    _ => return Err("Invalid puzzle description cut type"),
                };
                let distance = chunk[1]
                    .parse::<f64>()
                    .map_err(|_| "Invalid puzzle description cut distance")?;
                Ok(PuzzleCutDescription { cut_type, distance })
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        Ok(Self { shape, cuts })
    }
}
