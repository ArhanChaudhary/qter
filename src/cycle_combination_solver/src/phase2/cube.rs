//! A module providing functions to interact with the
//! structure and state of the Rubik's Cube.
//!
//! The state of the Rubik's Cube is internally represented
//! by four properties of the cube: corner permutation, corner
//! orientation, edge permutation, and edge orientation. A tuple
//! of these four properties (with correct parity relations)
//! uniquely determines the state of the cube.

use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use strum_macros::EnumString;

use super::CycleType;

/// An enum for the faces of the Rubik's Cube.
///
/// - U: top face
/// - D: bottom face
/// - L: left face
/// - R: right face
/// - F: front face
/// - B: back face
#[derive(PartialEq, Eq, EnumString, Debug, Clone, Copy)]
pub enum BaseMoveToken {
    U,
    D,
    L,
    R,
    F,
    B,
}

impl std::fmt::Display for BaseMoveToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Represents the direction which to turn a face. `Prime` represents
/// a counter-clockwise rotation of a face, and `Double` represents
/// a 180 degree rotation of a face.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Direction {
    Normal,
    Prime,
    Double,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Normal => write!(f, ""),
            Direction::Prime => write!(f, "'"),
            Direction::Double => write!(f, "2"),
        }
    }
}

/// An instantiation of a certain face equipped with a direction.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct MoveInstance {
    pub basemove: BaseMoveToken,
    pub dir: Direction,
}

impl MoveInstance {
    pub fn new(basemove: BaseMoveToken, dir: Direction) -> Self {
        Self { basemove, dir }
    }

    pub fn invert(&self) -> Self {
        Self {
            basemove: self.basemove,
            dir: match self.dir {
                Direction::Normal => Direction::Prime,
                Direction::Prime => Direction::Normal,
                x => x,
            },
        }
    }
}

impl std::fmt::Display for MoveInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.basemove, self.dir)
    }
}

/// A struct representing sequences of moves, used for representing
/// scramble sequences and solution sequences.
#[derive(Default)]
pub struct MoveSequence(Vec<MoveInstance>);

impl MoveSequence {
    pub fn from(vec: Vec<MoveInstance>) -> Self {
        Self(vec)
    }

    pub fn invert(&self) -> Self {
        let mut moves = vec![];
        for m in self.iter().rev() {
            moves.push(m.invert());
        }
        MoveSequence(moves)
    }

    /// Determines which moves are allowed after the given move sequence,
    /// to speed up solver methods.
    ///
    /// This is to avoid double rotations of faces (e.g. R R') and
    /// excessive rotations of antipodal faces (e.g. R L R can be simplified
    /// to R2 L).
    pub fn allowed_moves_after_seq(&self) -> u8 {
        match self.len() {
            0 => 0,
            1 => {
                let last_move = self[self.len() - 1];
                1 << get_basemove_pos(last_move.basemove)
            }
            _ => {
                let last_move = self[self.len() - 1];
                let second_to_last = self[self.len() - 2];
                if get_antipode(last_move.basemove) == second_to_last.basemove {
                    (1 << get_basemove_pos(last_move.basemove))
                        + (1 << get_basemove_pos(second_to_last.basemove))
                } else {
                    1 << get_basemove_pos(last_move.basemove)
                }
            }
        }
    }
}

impl Display for MoveSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut strs = vec![];
        for m in self.iter() {
            strs.push(m.to_string());
        }
        write!(f, "{}", strs.join(" "))
    }
}

impl Deref for MoveSequence {
    type Target = Vec<MoveInstance>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MoveSequence {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub const EDGES: usize = 12;
pub const CORNERS: usize = 8;

/// An internal set of permutation vectors representing what action
/// is done to a configuration of the Rubik's Cube when a move is applied.
///
/// The order of the corners and edges is as follows:
/// - Corners: UBL UBR UFR UFL DFL DFR DBR DBL
/// - Edges: UB UR UF UL BL BR FR FL DF DR DB DL
struct Move {
    cp_change: [u8; CORNERS], // a[i] gives the position that i goes to
    co_change: [i8; CORNERS],
    ep_change: [u8; EDGES],
    eo_change: [i8; EDGES],
}

/// A shorthand macro that can be used to construct MoveInstances.
///
/// ```
/// use rusty_rubik::cube::*;
/// use rusty_rubik::cube_move;
///
/// let r_prime: MoveInstance = cube_move!(R, Prime);
/// let u2: MoveInstance = cube_move!(U, Double);
/// ```
#[macro_export]
macro_rules! cube_move {
    ($basemove: ident, $dir:ident) => {{
        MoveInstance {
            basemove: BaseMoveToken::$basemove,
            dir: Direction::$dir,
        }
    }};
}

macro_rules! apply_permutation {
    ($og_state: expr, $delta: expr) => {{
        if $og_state.len() != $delta.len() {
            panic!("Size mismatch in applying permutation");
        } else {
            let mut new_array = $og_state.clone();
            for i in 0..$og_state.len() {
                new_array[$delta[i] as usize] = $og_state[i];
            }
            new_array
        }
    }};
}

macro_rules! apply_orientation {
    ($og_state: expr, $delta: expr, $num_orientations: expr) => {{
        if $og_state.len() != $delta.len() {
            panic!("Size mismatch in applying orientation");
        } else {
            let mut new_array = $og_state.clone();
            for i in 0..$og_state.len() {
                new_array[i] = (($og_state[i] + $delta[i] + $num_orientations) % $num_orientations);
                if new_array[i] == 2 {
                    new_array[i] = -1;
                }
            }
            new_array
        }
    }};
}

pub(crate) fn get_basemove_pos(token: BaseMoveToken) -> u8 {
    match token {
        BaseMoveToken::U => 5,
        BaseMoveToken::D => 4,
        BaseMoveToken::L => 3,
        BaseMoveToken::R => 2,
        BaseMoveToken::F => 1,
        BaseMoveToken::B => 0,
    }
}

fn get_antipode(token: BaseMoveToken) -> BaseMoveToken {
    match token {
        BaseMoveToken::U => BaseMoveToken::D,
        BaseMoveToken::D => BaseMoveToken::U,
        BaseMoveToken::L => BaseMoveToken::R,
        BaseMoveToken::R => BaseMoveToken::L,
        BaseMoveToken::F => BaseMoveToken::B,
        BaseMoveToken::B => BaseMoveToken::F,
    }
}

// bitvector: [UDLRFB], 0 means it's allowed
pub(crate) fn get_allowed_post_moves(prev_bv: u8, last_move: Option<BaseMoveToken>) -> u8 {
    if let Some(lm) = last_move {
        let antipode = get_antipode(lm);
        if prev_bv & (1 << get_basemove_pos(antipode)) != 0 {
            // then the antipode was already applied
            (1 << get_basemove_pos(lm)) + (1 << get_basemove_pos(antipode))
        } else {
            1 << get_basemove_pos(lm)
        }
    } else {
        0
    }
}

/// The underlying struct for representing a configuration of the Rubik's Cube.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CubeState {
    cp: [u8; CORNERS],
    co: [i8; CORNERS],
    ep: [u8; EDGES],
    eo: [i8; EDGES],
}

impl Default for CubeState {
    fn default() -> CubeState {
        CubeState {
            cp: const {
                let mut arr = [0; CORNERS];
                let mut i = 0;
                while i < CORNERS {
                    arr[i] = i as u8;
                    i += 1;
                }
                arr
            },
            co: [0_i8; CORNERS],
            ep: const {
                let mut arr = [0; EDGES];
                let mut i = 0;
                while i < EDGES {
                    arr[i] = i as u8;
                    i += 1;
                }
                arr
            },
            eo: [0_i8; EDGES],
        }
    }
}

fn get_move_matrix(mov: &BaseMoveToken) -> Move {
    match mov {
        BaseMoveToken::U => MOVE_U,
        BaseMoveToken::D => MOVE_D,
        BaseMoveToken::L => MOVE_L,
        BaseMoveToken::R => MOVE_R,
        BaseMoveToken::F => MOVE_F,
        BaseMoveToken::B => MOVE_B,
    }
}

fn factorial(num: u32) -> u32 {
    match num {
        0 => 1,
        1 => 1,
        _ => factorial(num - 1) * num,
    }
}

// range:
// corners: [0, 8! - 1]
// edges: [0, 12! - 1]
fn get_index_of_permutation(perm: &[u8]) -> u32 {
    // 2 bytes suffice for 12!
    let mut fin = 0;
    for i in 0..perm.len() {
        let mut res = 0;
        for j in (i + 1)..perm.len() {
            if perm[j] < perm[i] {
                res += 1;
            }
        }
        fin += res * factorial((perm.len() - i - 1) as u32);
    }
    fin
}

// range:
// corners: [0, 3^7 - 1]
// edges: [0, 2^11 - 1]
fn get_index_of_orientation(ori: &[i8], num_orientations: u8) -> u16 {
    let mut result = 0;
    for (i, val) in ori.iter().enumerate() {
        if i == ori.len() - 1 {
            break;
        }
        let pos = (val + num_orientations as i8) % num_orientations as i8;
        result += pos as u16;
        if i != ori.len() - 2 {
            result *= num_orientations as u16;
        }
    }
    result
}

pub fn induces_oriented_partition(
    perm: &[u8],
    ori: &[i8],
    // cycle_type: &CycleType<u8>,
    partition: &[(u8, bool)],
    orientation_count: i8,
    multi_bv: &mut [u8],
) -> bool {
    // TODO: get this working for any piece orbit
    // reuse memory
    multi_bv.fill(0);
    // visited corners is LSB, covered_cycle_lengths is 2nd LSB
    let mut covered_cycles_count = 0;
    for i in 0..perm.len() {
        if multi_bv[i] & 1 != 0 {
            continue;
        }
        multi_bv[i] |= 1;
        let mut actual_cycle_length = 1;
        let mut corner = perm[i] as usize;
        let mut orientation_sum = ori[corner];

        while corner != i {
            actual_cycle_length += 1;
            multi_bv[corner] |= 1;
            corner = perm[corner] as usize;
            orientation_sum += ori[corner];
        }

        let actual_orients = orientation_sum % orientation_count != 0;
        if actual_cycle_length == 1 && !actual_orients {
            continue;
        }
        let Some(valid_cycle_index) = partition.iter().enumerate().position(
            |(j, &(expected_cycle_length, expected_orients))| {
                expected_cycle_length == actual_cycle_length
                    && expected_orients == actual_orients
                    && (multi_bv[j] & 2 == 0)
            },
        ) else {
            return false;
        };
        multi_bv[valid_cycle_index] |= 2;
        covered_cycles_count += 1;
        // cannot possibly return true if this runs
        if covered_cycles_count > partition.len() {
            return false;
        }
    }
    covered_cycles_count == partition.len()
}

impl CubeState {
    pub fn from_corners(cp: [u8; CORNERS], co: [i8; CORNERS]) -> Self {
        CubeState {
            cp,
            co,
            ..Default::default()
        }
    }

    fn apply_basemove(&self, m: &BaseMoveToken) -> Self {
        let mov = get_move_matrix(m);
        let oriented_corners = apply_orientation!(&self.co, &mov.co_change, 3);
        let oriented_edges = apply_orientation!(&self.eo, &mov.eo_change, 2);
        CubeState {
            cp: apply_permutation!(&self.cp, &mov.cp_change),
            co: apply_permutation!(oriented_corners, &mov.cp_change),
            ep: apply_permutation!(&self.ep, &mov.ep_change),
            eo: apply_permutation!(oriented_edges, &mov.ep_change),
        }
    }

    /// Applies a move to a Rubik's Cube configuration.
    pub fn apply_move_instance(&self, m: &MoveInstance) -> Self {
        let num_turns = match &m.dir {
            Direction::Normal => 1,
            Direction::Prime => 3,
            Direction::Double => 2,
        };
        (0..num_turns).fold(self.clone(), |acc, _| acc.apply_basemove(&m.basemove))
    }

    /// Applies a sequence of moves, in order to a Rubik's Cube configuration.
    pub fn apply_move_instances(&self, moves: &MoveSequence) -> Self {
        moves
            .iter()
            .fold(self.clone(), |acc, mov| acc.apply_move_instance(mov))
    }

    pub fn corner_state_index(&self) -> u32 {
        let cp_index = get_index_of_permutation(&self.cp);
        let co_index = get_index_of_orientation(&self.co, 3);
        cp_index * u32::pow(3, 7) + (co_index as u32)
    }

    pub fn induces_corner_cycle_type(
        &self,
        cycle_type: &CycleType<u8>,
        multi_bv: &mut [u8],
    ) -> bool {
        induces_oriented_partition(
            &self.cp,
            &self.co,
            &cycle_type.corner_partition,
            3,
            multi_bv,
        )
    }

    pub fn induces_cycle_type(&self, cycle_type: &CycleType<u8>, multi_bv: &mut [u8]) -> bool {
        self.induces_corner_cycle_type(cycle_type, multi_bv)
            && induces_oriented_partition(
                &self.ep,
                &self.eo,
                &cycle_type.edge_partition,
                2,
                multi_bv,
            )
    }
}

/// A vector of all allowed moves on a Rubik's Cube.
pub const ALL_MOVES: [MoveInstance; 18] = [
    cube_move!(U, Normal),
    cube_move!(U, Prime),
    cube_move!(U, Double),
    cube_move!(D, Normal),
    cube_move!(D, Prime),
    cube_move!(D, Double),
    cube_move!(L, Normal),
    cube_move!(L, Prime),
    cube_move!(L, Double),
    cube_move!(R, Normal),
    cube_move!(R, Prime),
    cube_move!(R, Double),
    cube_move!(F, Normal),
    cube_move!(F, Prime),
    cube_move!(F, Double),
    cube_move!(B, Normal),
    cube_move!(B, Prime),
    cube_move!(B, Double),
];

const MOVE_U: Move = Move {
    cp_change: [1, 2, 3, 0, 4, 5, 6, 7],
    co_change: [0, 0, 0, 0, 0, 0, 0, 0],
    ep_change: [1, 2, 3, 0, 4, 5, 6, 7, 8, 9, 10, 11],
    eo_change: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
};

const MOVE_D: Move = Move {
    cp_change: [0, 1, 2, 3, 5, 6, 7, 4],
    co_change: [0, 0, 0, 0, 0, 0, 0, 0],
    ep_change: [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 8],
    eo_change: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
};

const MOVE_R: Move = Move {
    cp_change: [0, 6, 1, 3, 4, 2, 5, 7],
    co_change: [0, -1, 1, 0, 0, -1, 1, 0],
    ep_change: [0, 5, 2, 3, 4, 9, 1, 7, 8, 6, 10, 11],
    eo_change: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
};

const MOVE_L: Move = Move {
    cp_change: [3, 1, 2, 4, 7, 5, 6, 0],
    co_change: [1, 0, 0, -1, 1, 0, 0, -1],
    ep_change: [0, 1, 2, 7, 3, 5, 6, 11, 8, 9, 10, 4],
    eo_change: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
};

const MOVE_F: Move = Move {
    cp_change: [0, 1, 5, 2, 3, 4, 6, 7],
    co_change: [0, 0, -1, 1, -1, 1, 0, 0],
    ep_change: [0, 1, 6, 3, 4, 5, 8, 2, 7, 9, 10, 11],
    eo_change: [0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0],
};

const MOVE_B: Move = Move {
    cp_change: [7, 0, 2, 3, 4, 5, 1, 6],
    co_change: [-1, 1, 0, 0, 0, 0, -1, 1],
    ep_change: [4, 1, 2, 3, 10, 0, 6, 7, 8, 9, 5, 11],
    eo_change: [1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::{cube, parser};
    use itertools::{repeat_n, Itertools};

    #[test]
    fn test_custom_permutation_index() {
        for cubies in 1..=10 {
            for (i, perm) in (0..cubies).permutations(cubies as usize).enumerate() {
                let left = get_index_of_permutation(&perm);
                let right = i as u32;

                assert_eq!(left, right);
            }
        }
    }

    #[test]
    fn test_custom_orientation_index() {
        let k = 8;
        for orientation_count in 1..=4 {
            for (i, orients) in repeat_n(0..orientation_count, k)
                .multi_cartesian_product()
                .filter(|p| p.iter().sum::<i8>().rem_euclid(orientation_count) == 0)
                .enumerate()
            {
                let left = get_index_of_orientation(&orients, orientation_count as u8);
                let right = i as u16;
                assert_eq!(left, right);
            }
        }
    }

    fn induces_corner_cycle_type(
        scramble: &str,
        cycle_type: CycleType<u8>,
        multi_bv: &mut [u8],
    ) -> bool {
        let parsed_seq = parser::parse_scramble(scramble).unwrap();
        let seq = MoveSequence(parsed_seq);
        let state = CubeState::default().apply_move_instances(&seq);
        state.induces_corner_cycle_type(&cycle_type, multi_bv)
    }

    #[test]
    fn test_induces_oriented_partition() {
        let mut multi_bv = vec![0_u8; EDGES.max(CORNERS)];

        assert!(!induces_oriented_partition(
            &[0, 1, 2, 3, 8, 5, 6, 9, 4, 7, 10, 11],
            &[0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0],
            &[(2, true), (2, true)],
            2,
            &mut multi_bv,
        ));
    }

    #[test]
    fn test_induces_corner_cycle_type_all_orients() {
        // we can guarantee the partition length will never be greater than the number of pieces in the orbit
        let mut multi_bv = vec![0_u8; EDGES.max(CORNERS)];
        assert!(induces_corner_cycle_type(
            "F2 L' U2 F U F U L' B U' F' U D2 L F2 B'",
            CycleType {
                corner_partition: vec![(1, true), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "F2 L' U2 F2 U L' U' F' U2 B D2 L F2 B'",
            CycleType {
                corner_partition: vec![(1, true), (1, true), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "U2 L B L2 F U2 B' U2 R U' F R' F' R F' L' U2",
            CycleType {
                corner_partition: vec![(1, true), (5, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "R' U2 R' U2 F' D' L F L2 F U2 F2 D' L' D2 F R2",
            CycleType {
                corner_partition: vec![(1, true), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "B2 U' B' D B' L' D' B U' R2 B2 R U B2 R B' R U",
            CycleType {
                corner_partition: vec![(1, true), (1, true), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "R2 L2 D' B L2 D' B L' B D2 R2 B2 R' D' B2 L2 U'",
            CycleType {
                corner_partition: vec![(2, true), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "F' B2 R L U2 B U2 L2 F2 U R L B' L' D' R' D' B'",
            CycleType {
                corner_partition: vec![(1, true), (2, true), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "L' D2 F B2 U F' L2 B R F2 D R' L F R' F' D",
            CycleType {
                corner_partition: vec![(2, true), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "B' L' F2 R U' R2 F' L2 F R' L B L' U' F2 U' D2 L",
            CycleType {
                corner_partition: vec![(1, true), (2, true), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));
    }

    #[test]
    fn test_induces_cycle_type_mixed_orients() {
        let mut multi_bv = vec![0_u8; cube::CORNERS.max(cube::EDGES)];
        assert!(induces_corner_cycle_type(
            "F2 D2 L' F D R2 F2 U2 L2 F R' B2 D2 R2 U R2 U",
            CycleType {
                corner_partition: vec![(1, true), (2, false), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "F2 B' R' F' L' D B' U' F U B' U2 D L' F' L' B R2",
            CycleType {
                corner_partition: vec![(1, true), (2, false), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "U L U L2 U2 B2",
            CycleType {
                corner_partition: vec![(1, true), (2, false), (3, true)],
                ..Default::default()
            },
            &mut multi_bv
        ));

        assert!(induces_corner_cycle_type(
            "U",
            CycleType {
                corner_partition: vec![(4, false)],
                ..Default::default()
            },
            &mut multi_bv
        ));
    }
}
