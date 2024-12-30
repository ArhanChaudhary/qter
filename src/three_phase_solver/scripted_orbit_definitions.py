
from common_types import (
    PuzzleOrbitDefinition,
    OrientationSumConstraint,
    OrientationStatus,
    Orbit,
    EvenParityConstraint,
)


def NxNDefinitions(N):
    PUZZLE_NxN = PuzzleOrbitDefinition(
        orbits=[
            Orbit(
                name="corners",
                cubie_count=8,
                orientation_status=OrientationStatus.CanOrient(
                    count=3,
                    sum_constraint=OrientationSumConstraint.ZERO,
                ),
            ),
        ],
        even_parity_constraints=set(),
    )

    center_begin = 1
    if N % 2 == 1:
        center_begin = 0
        PUZZLE_NxN.orbits.append(
            Orbit(
                name="edges",
                cubie_count=12,
                orientation_status=OrientationStatus.CanOrient(
                    count=2,
                    sum_constraint=OrientationSumConstraint.ZERO,
                ),
            ),
        )


    for w in range(1,N//2):
        PUZZLE_NxN.orbits.append(
            Orbit(
                name="wings"+str(w),
                cubie_count=24,
                orientation_status=OrientationStatus.CannotOrient(),
            ),
        )
    for c1 in range(center_begin,N//2):
        for c2 in range(1,N//2):
            PUZZLE_NxN.orbits.append(
                Orbit(
                    name="centers"+str(c1)+";"+str(c2),
                    cubie_count=24,
                    orientation_status=OrientationStatus.CannotOrient(),
                ),
            )
    
    if N % 2 == 1:
        PUZZLE_NxN.even_parity_constraints.add(
            EvenParityConstraint(
                orbit_names=("edges", "corners"),
            ),
        )
        for c2 in range(1,N//2):
            PUZZLE_NxN.even_parity_constraints.add(
                EvenParityConstraint(
                    orbit_names=("corners", "wings"+str(c2), "centers0;"+str(c2)),
                ),
            )

    for c1 in range(1,N//2):
        PUZZLE_NxN.even_parity_constraints.add(
            EvenParityConstraint(
                orbit_names=("corners", "centers"+str(c1)+";"+str(c1)),
            ),
        )
        for c2 in range(1,N//2):
            if c1 == c2:
                continue
            PUZZLE_NxN.even_parity_constraints.add(
                EvenParityConstraint(
                    orbit_names=("corners", "wings"+str(c1), "wings"+str(c2), "centers"+str(c1)+";"+str(c2)),
                ),
            )
    return PUZZLE_NxN


def NMinxDefinitions(N):
    PUZZLE_NMinx = PuzzleOrbitDefinition(
        orbits=[
            Orbit(
                name="corners",
                cubie_count=20,
                orientation_status=OrientationStatus.CanOrient(
                    count=3,
                    sum_constraint=OrientationSumConstraint.ZERO,
                ),
            ),
        ],
        even_parity_constraints=set(),
    )

    PUZZLE_NMinx.even_parity_constraints.add(
        EvenParityConstraint(
            orbit_names=("corners"),
        ),
    )

    center_begin = 1
    if N % 2 == 1:
        center_begin = 0
        PUZZLE_NMinx.orbits.append(
            Orbit(
                name="edges",
                cubie_count=30,
                orientation_status=OrientationStatus.CanOrient(
                    count=2,
                    sum_constraint=OrientationSumConstraint.ZERO,
                ),
            ),
        )
        PUZZLE_NMinx.even_parity_constraints.add(
            EvenParityConstraint(
                orbit_names=("edges"),
            ),
        )


    for w in range(1,N//2):
        PUZZLE_NMinx.orbits.append(
            Orbit(
                name="wings"+str(w),
                cubie_count=60,
                orientation_status=OrientationStatus.CannotOrient(),
            ),
        )
        PUZZLE_NMinx.even_parity_constraints.add(
            EvenParityConstraint(
                orbit_names=("wings"+str(w)),
            ),
        )
    for c1 in range(center_begin,N//2):
        for c2 in range(1,N//2):
            PUZZLE_NMinx.orbits.append(
                Orbit(
                    name="centers"+str(c1)+";"+str(c2),
                    cubie_count=60,
                    orientation_status=OrientationStatus.CannotOrient(),
                ),
            )
            PUZZLE_NMinx.even_parity_constraints.add(
                EvenParityConstraint(
                    orbit_names=("centers"+str(c1)+";"+str(c2)),
                ),
            )
    
    return PUZZLE_NMinx

PUZZLE_2x2 = NxNDefinitions(2)
PUZZLE_3x3 = NxNDefinitions(3)
PUZZLE_4x4 = NxNDefinitions(4)
PUZZLE_5x5 = NxNDefinitions(5)
PUZZLE_6x6 = NxNDefinitions(6)
PUZZLE_KILOMINX = NMinxDefinitions(2)
PUZZLE_MEGAMINX = NMinxDefinitions(3)
PUZZLE_MASTERKILOMINX = NMinxDefinitions(4)
PUZZLE_GIGAMINX = NMinxDefinitions(5)