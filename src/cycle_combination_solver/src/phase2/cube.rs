impl CubeState {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::{cube, parser};
    use itertools::{repeat_n, Itertools};

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
