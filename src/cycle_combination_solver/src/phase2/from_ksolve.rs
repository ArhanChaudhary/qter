use super::puzzle::{Move, OrbitDef, PuzzleDef, PuzzleState};
use puzzle_geometry::{KSolve, KSolveMove, KSolveSet};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KSolveConversionError {
    #[error("Phase 2 does not currently support puzzles with set sizes larger than 255, but it will in the future")]
    SetSizeTooBig,
    #[error("Invalid set count, expected {0} sets but got {1}")]
    InvalidSetCount(usize, usize),
    #[error("Invalid piece count, expected {0} pieces but got {1}")]
    InvalidPieceCount(u8, usize),
    #[error("Invalid orientation count, expected a maximum of {0} orientations but got {1}")]
    InvalidOrientation(u8, u8),
    #[error("Permutation out of range, expected a value between 0 and 255 but got {0}")]
    PermutationOutOfRange(u16),
    #[error("Move is invalid: {0:?}")]
    InvalidMove(KSolveMove),
}

impl<P: PuzzleState<Error = KSolveConversionError>> TryFrom<&KSolve> for PuzzleDef<P> {
    type Error = KSolveConversionError;

    fn try_from(ksolve: &KSolve) -> Result<Self, Self::Error> {
        let orbit_defs: Vec<OrbitDef> = ksolve
            .sets()
            .iter()
            .map(|ksolve_set| {
                Ok(OrbitDef {
                    piece_count: ksolve_set
                        .piece_count()
                        .try_into()
                        .map_err(|_| KSolveConversionError::SetSizeTooBig)?,
                    orientation_count: ksolve_set.orientation_count(),
                })
            })
            .collect::<Result<_, KSolveConversionError>>()?;

        let expected_set_count = ksolve.sets().len();
        let moves: Vec<Move<P>> = ksolve
            .moves()
            .iter()
            .map(|ksolve_move| {
                let actual_set_count = ksolve_move.transformation().len();

                if actual_set_count != expected_set_count {
                    return Err(KSolveConversionError::InvalidSetCount(
                        orbit_defs.len(),
                        ksolve_move.transformation().len(),
                    ));
                }

                for (transformation, orbit_def) in ksolve_move
                    .zero_indexed_transformation()
                    .iter()
                    .zip(&orbit_defs)
                {
                    let expected_piece_count = orbit_def.piece_count;
                    let actual_piece_count = transformation.len();

                    if actual_piece_count != expected_piece_count as usize {
                        return Err(KSolveConversionError::InvalidPieceCount(
                            expected_piece_count,
                            actual_piece_count,
                        ));
                    }

                    let max_orientation = orbit_def.orientation_count;
                    let mut covered_perms = vec![false; expected_piece_count as usize];

                    for &(perm, orientation) in transformation {
                        if orientation >= max_orientation {
                            return Err(KSolveConversionError::InvalidOrientation(
                                max_orientation,
                                orientation,
                            ));
                        }
                        match covered_perms.get_mut(perm as usize) {
                            Some(i) => *i = true,
                            None => return Err(KSolveConversionError::PermutationOutOfRange(perm)),
                        }
                    }

                    if covered_perms.iter().any(|&x| !x) {
                        return Err(KSolveConversionError::InvalidMove(ksolve_move.clone()));
                    }
                }
                Ok(Move {
                    name: ksolve_move.name().to_owned(),
                    transformation: ksolve_move.try_into()?,
                })
            })
            .collect::<Result<_, KSolveConversionError>>()?;
        Ok(PuzzleDef {
            name: ksolve.name().to_owned(),
            moves,
            orbit_defs,
        })
    }
}

pub fn slice_try_from_ksolve(
    ksolve_move: &KSolveMove,
    buf: &mut [u8],
) -> Result<(), KSolveConversionError> {
    let mut i = 0;
    for transformation in ksolve_move.zero_indexed_transformation() {
        let size = transformation.len();
        for j in 0..size {
            let (perm, orientation) = transformation[j];
            *buf.get_mut(i + j + size)
                .ok_or_else(|| KSolveConversionError::InvalidMove(ksolve_move.clone()))? =
                orientation;
            buf[i + j] = perm
                .try_into()
                .map_err(|_| KSolveConversionError::PermutationOutOfRange(perm))?;
        }
        i += size * 2;
    }
    Ok(())
}
