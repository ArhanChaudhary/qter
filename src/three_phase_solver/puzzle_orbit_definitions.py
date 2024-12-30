from common_types import (
    PuzzleOrbitDefinition,
    OrientationSumConstraint,
    OrientationStatus,
    Orbit,
    EvenParityConstraint,
)

PUZZLE_3x3 = PuzzleOrbitDefinition(
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
    even_parity_constraints=(
        EvenParityConstraint(
            orbit_names=("edges", "corners"),
        ),
    ),
)

PUZZLE_4x4 = PuzzleOrbitDefinition(
    orbits=(
        Orbit(
            name="corners",
            cubie_count=8,
            orientation_status=OrientationStatus.CanOrient(
                count=3,
                sum_constraint=OrientationSumConstraint.ZERO,
            ),
        ),
        Orbit(
            name="wings",
            cubie_count=24,
            orientation_status=OrientationStatus.CannotOrient(),
        ),
        Orbit(
            name="centers",
            cubie_count=24,
            orientation_status=OrientationStatus.CannotOrient(),
        ),
    ),
    even_parity_constraints=(
        EvenParityConstraint(
            orbit_names=("corners", "centers"),
        ),
    ),
)


PUZZLE_5x5 = PuzzleOrbitDefinition(
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
        Orbit(
            name="wings",
            cubie_count=24,
            orientation_status=OrientationStatus.CannotOrient(),
        ),
        Orbit(
            name="xcenters",
            cubie_count=24,
            orientation_status=OrientationStatus.CannotOrient(),
        ),
        Orbit(
            name="+centers",
            cubie_count=24,
            orientation_status=OrientationStatus.CannotOrient(),
        ),
    ),
    even_parity_constraints=(
        EvenParityConstraint(
            orbit_names=("edges", "corners"),
        ),
        EvenParityConstraint(
            orbit_names=("corners", "xcenters"),
        ),
        EvenParityConstraint(
            orbit_names=("corners", "wings", "+centers"),
        ),
    ),
)

PUZZLE_MEGAMINX = PuzzleOrbitDefinition(
    orbits=(
        Orbit(
            name="edges",
            cubie_count=30,
            orientation_status=OrientationStatus.CanOrient(
                count=2,
                sum_constraint=OrientationSumConstraint.ZERO,
            ),
        ),
        Orbit(
            name="corners",
            cubie_count=20,
            orientation_status=OrientationStatus.CanOrient(
                count=3,
                sum_constraint=OrientationSumConstraint.ZERO,
            ),
        ),
    ),
    even_parity_constraints=(
        EvenParityConstraint(
            orbit_names=("edges",),
        ),
        EvenParityConstraint(
            orbit_names=("corners",),
        ),
    ),
)
