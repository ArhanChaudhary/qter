import unittest
import puzzle_orbit_definitions
import phase1
from common_types import (
    PuzzleOrbitDefinition,
    Orbit,
    OrientationStatus,
    OrientationSumConstraint,
    EvenParityConstraint,
)

unittest.util._MAX_LENGTH = 500


class Puzzle_3x3(unittest.TestCase):
    def test_3x3_1_cycle(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=puzzle_orbit_definitions.PUZZLE_3x3,
            num_cycles=1,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {
                (1260,): 2,
            },
        )

    def test_3x3_2_cycles(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=puzzle_orbit_definitions.PUZZLE_3x3,
            num_cycles=2,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {
                (90, 90): 16,
                (630, 9): 4,
                (180, 30): 1,
                (210, 24): 1,
                (126, 36): 8,
                (360, 12): 4,
                (720, 2): 2,
            },
        )

    def test_3x3_3_cycles(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=puzzle_orbit_definitions.PUZZLE_3x3,
            num_cycles=3,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {
                (90, 90, 6): 1,
                (90, 30, 18): 1,
                (30, 30, 30): 2,
                (180, 18, 6): 2,
                (126, 12, 12): 1,
                (630, 9, 3): 1,
                (210, 9, 9): 1,
                (36, 36, 12): 1,
                (126, 36, 3): 2,
                (42, 36, 9): 2,
                (360, 6, 6): 4,
                (210, 15, 3): 1,
            },
        )

    def test_3x3_4_cycles(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=puzzle_orbit_definitions.PUZZLE_3x3,
            num_cycles=4,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {
                (90, 24, 6, 6): 1,
                (30, 24, 18, 6): 1,
                (126, 12, 6, 6): 1,
                (42, 18, 12, 6): 1,
                (30, 12, 12, 12): 1,
                (90, 90, 3, 2): 1,
                (90, 30, 9, 2): 1,
                (90, 30, 6, 3): 8,
                (90, 18, 10, 3): 1,
                (90, 10, 9, 6): 1,
                (30, 30, 18, 3): 8,
                (30, 30, 9, 6): 8,
                (30, 18, 10, 9): 1,
                (126, 18, 6, 3): 1,
                (90, 36, 6, 2): 2,
                (90, 18, 12, 2): 2,
                (90, 12, 12, 3): 2,
                (36, 30, 18, 2): 2,
                (36, 30, 12, 3): 2,
                (36, 30, 6, 6): 16,
                (18, 18, 12, 10): 2,
                (126, 24, 3, 3): 1,
                (42, 24, 9, 3): 1,
                (42, 18, 18, 2): 5,
                (60, 45, 3, 3): 1,
                (36, 36, 6, 3): 4,
                (210, 6, 6, 3): 1,
                (180, 18, 3, 2): 2,
                (180, 12, 3, 3): 2,
                (180, 9, 6, 2): 2,
                (630, 3, 3, 3): 6,
                (210, 9, 3, 3): 7,
                (360, 6, 3, 2): 4,
                (210, 12, 2, 2): 1,
            },
        )


@unittest.skip("4x4 is too slow")
class Puzzle_4x4(unittest.TestCase):
    def test_4x4_1_cycle(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=puzzle_orbit_definitions.PUZZLE_4x4,
            num_cycles=1,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {
                (765765,): 2,
            },
        )

    def test_4x4_2_cycles(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=puzzle_orbit_definitions.PUZZLE_4x4,
            num_cycles=2,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        breakpoint()
        self.assertEqual(
            stats,
            {
                (90, 90): 16,
                (630, 9): 4,
                (180, 30): 1,
                (210, 24): 1,
                (126, 36): 8,
                (360, 12): 4,
                (720, 2): 2,
            },
        )


class NoConstraints(unittest.TestCase):
    def test_3x3_1_cycle_no_parity_constraints(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=puzzle_orbit_definitions.PuzzleOrbitDefinition(
                orbits=(
                    Orbit(
                        name="edges",
                        cubie_count=12,
                        orientation_status=OrientationStatus.CanOrient(
                            count=2,
                            sum_constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                    Orbit(
                        name="corners",
                        cubie_count=8,
                        orientation_status=OrientationStatus.CanOrient(
                            count=3,
                            sum_constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                ),
                even_parity_constraints=(),
            ),
            num_cycles=2,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {
                (360, 36): 8,
                (180, 72): 8,
                (90, 90): 16,
                (630, 12): 1,
                (1260, 4): 2,
                (840, 6): 2,
            },
        )

    def test_3x3_2_cycles_no_orientation_constraints(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=PuzzleOrbitDefinition(
                orbits=(
                    Orbit(
                        name="edges",
                        cubie_count=12,
                        orientation_status=OrientationStatus.CanOrient(
                            count=2,
                            sum_constraint=OrientationSumConstraint.NONE,
                        ),
                    ),
                    Orbit(
                        name="corners",
                        cubie_count=8,
                        orientation_status=OrientationStatus.CanOrient(
                            count=3,
                            sum_constraint=OrientationSumConstraint.NONE,
                        ),
                    ),
                ),
                even_parity_constraints=(
                    EvenParityConstraint(
                        orbit_names=("edges", "corners"),
                    ),
                ),
            ),
            num_cycles=2,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {(210, 90): 1, (630, 15): 1, (360, 18): 6, (720, 2): 2},
        )

    def test_3x3_2_cycles_no_parity_constraints(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=puzzle_orbit_definitions.PuzzleOrbitDefinition(
                orbits=(
                    Orbit(
                        name="edges",
                        cubie_count=12,
                        orientation_status=OrientationStatus.CanOrient(
                            count=2,
                            sum_constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                    Orbit(
                        name="corners",
                        cubie_count=8,
                        orientation_status=OrientationStatus.CanOrient(
                            count=3,
                            sum_constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                ),
                even_parity_constraints=(),
            ),
            num_cycles=2,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {
                (360, 36): 8,
                (180, 72): 8,
                (90, 90): 16,
                (630, 12): 1,
                (1260, 4): 2,
                (840, 6): 2,
            },
        )

    def test_3x3_2_cycles_no_orientation_constraints_and_no_parity_constraints(self):
        cycle_combination_objs = phase1.optimal_cycle_combinations(
            puzzle_orbit_definition=PuzzleOrbitDefinition(
                orbits=(
                    Orbit(
                        name="edges",
                        cubie_count=12,
                        orientation_status=OrientationStatus.CanOrient(
                            count=2,
                            sum_constraint=OrientationSumConstraint.NONE,
                        ),
                    ),
                    Orbit(
                        name="corners",
                        cubie_count=8,
                        orientation_status=OrientationStatus.CanOrient(
                            count=3,
                            sum_constraint=OrientationSumConstraint.NONE,
                        ),
                    ),
                ),
                even_parity_constraints=(),
            ),
            num_cycles=2,
        )
        stats = phase1.cycle_combination_objs_stats(cycle_combination_objs)
        self.assertEqual(
            stats,
            {
                (360, 60): 4,
                (180, 120): 4,
                (210, 90): 1,
                (240, 72): 1,
                (420, 36): 4,
                (630, 18): 1,
                (1260, 6): 1,
                (840, 9): 2,
            },
        )


if __name__ == "__main__":
    unittest.main()
