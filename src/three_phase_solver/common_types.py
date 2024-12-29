import collections
import dataclasses
import enum


class OrientationSumConstraint(enum.Enum):
    ZERO = enum.auto()
    NONE = enum.auto()


class OrientationStatus:
    @dataclasses.dataclass(frozen=True)
    class CannotOrient:
        pass

    @dataclasses.dataclass(frozen=True)
    class CanOrient:
        count: int
        sum_constraint: OrientationSumConstraint

        def __hash__(self):
            return hash((self.count, self.sum_constraint))


PuzzleOrbitDefinition = collections.namedtuple(
    "PuzzleOrbitDefinition",
    [
        "orbits",
        "even_parity_constraints",
    ],
)


Orbit = collections.namedtuple(
    "Orbit",
    [
        "name",
        "cubie_count",
        "orientation_status",
    ],
)

EvenParityConstraint = collections.namedtuple(
    "EqualPermutationCombinations",
    [
        "orbit_names",
    ],
)
