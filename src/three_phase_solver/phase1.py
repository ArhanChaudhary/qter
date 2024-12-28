"""
Phase 1 of the three-phase solver.

This phase is responsible for finding pairs of commutative cycles on a Rubik's cube
that have high products of orders. The output of this phase is directly
used in phase 2.

Adapted with permission from ScriptRaccon's
<https://gist.github.com/ScriptRaccoon/c12c4884c116dead62a15a3d09732d5d>
"""

import collections
import dataclasses
import itertools
import math
import operator
import functools
import timeit
import enum


class ShareState(enum.Enum):
    FREE = enum.auto()
    CANNOT_SHARE_ORIENTATION = enum.auto()
    MUST_SHARE_ORIENTATION = enum.auto()


class OrientationSumConstraint(enum.Enum):
    ZERO = enum.auto()
    NONE = enum.auto()


class OrientationFactor:
    @dataclasses.dataclass
    class One:
        pass

    @dataclasses.dataclass
    class GtOne:
        factor: int
        constraint: OrientationSumConstraint

        def __hash__(self):
            return hash((self.factor, self.constraint))


PuzzleOrbitDefinition = collections.namedtuple(
    "PuzzleOrbitDefinition",
    [
        "orbits",
        "even_permutation_combinations",
    ],
)


Orbit = collections.namedtuple(
    "Orbit",
    [
        "name",
        "cubie_count",
        "orientation_factor",
    ],
)


CycleCombination = collections.namedtuple(
    "CycleCombination",
    [
        "used_cubie_counts",
        "order_product",
        "share_orders",
        "cycle_combination",
    ],
)


Cycle = collections.namedtuple(
    "Cycle",
    [
        "order",
        "share",
        "partition_objs",
    ],
)


CubiePartition = collections.namedtuple(
    "CubiePartition",
    [
        "name",
        "partition",
        "order",
        "always_orient",
        "critical_orient",
    ],
)


@functools.cache
def integer_partitions(n):
    """
    Find the [integer partition](https://en.wikipedia.org/wiki/Integer_partition)
    of n.

    Intuitively, this represents the permutation orders of all possible sets of
    cycles that use n cubies, ignoring parity and orientation concerns.

    Group theory-wise, the positions of edge and corner pieces are isomorphic to
    the symmetric groups of size 12 and 8 respectively. Since the conjugacy
    classes of a symmetric group [correspond](https://en.wikipedia.org/wiki/Symmetric_group#Conjugacy_classes)
    to integer partitions, this can also be thought of as a representation of the
    conjugacy classes of those symmetric groups.

    Taken from <https://stackoverflow.com/a/10036764/12230735>.
    """
    if n == 0:
        return {()}
    answer = {(n,)}
    for x in range(1, n):
        for y in integer_partitions(n - x):
            answer.add(tuple(sorted((x,) + y)))
    return answer


# https://stackoverflow.com/a/6285330/12230735
def unique_permutations(iterable, r=None):
    previous = ()
    for p in itertools.permutations(sorted(iterable), r):
        if p > previous:
            previous = p
            yield p


def p_adic_valuation(n, p):
    """
    Calculate the [p-adic valuation](https://en.wikipedia.org/wiki/P-adic_valuation).
    """
    exponent = 0
    while n % p == 0 and n != 0:
        n //= p
        exponent += 1
    return exponent


def sign(partition):
    """
    Calculate the [signature](https://en.wikipedia.org/wiki/Parity_of_a_permutation)
    of a partition, made easy by having all cycle lengths.
    """
    return (sum(partition) - len(partition)) % 2


def cycle_combination_dominates(this, other):
    # A modification of the weakly dominates condition in the pareto efficient
    # algorithm
    different_orders = False
    same_cycle = this.share_orders == other.share_orders
    for this_cycle, other_cycle in zip(this.cycle_combination, other.cycle_combination):
        if other_cycle.order > this_cycle.order:
            return False
        if different_orders:
            continue
        different_orders |= this_cycle.order > other_cycle.order
        same_cycle &= all(
            this_cubie_partition == other_cubie_partition
            for this_cubie_partition, other_cubie_partition in zip(
                map(
                    operator.attrgetter("partition"),
                    this_cycle.partition_objs,
                ),
                map(
                    operator.attrgetter("partition"),
                    other_cycle.partition_objs,
                ),
            )
        )

    return different_orders or same_cycle


def all_cycle_combinations(puzzle_orbit_definition, num_cycles):
    """
    Finds all cycle structure pairings on the Rubik's cube.
    """
    global puzzle_orbit_definition_global
    global even_permutation_combinations_indicies_global
    puzzle_orbit_definition_global = puzzle_orbit_definition
    even_permutation_combinations_indicies_global = tuple(
        tuple(
            next(
                i
                for i, orbit in enumerate(puzzle_orbit_definition.orbits)
                if orbit.name == orbit_name
            )
            for orbit_name in even_permutation_combination
        )
        for even_permutation_combination in puzzle_orbit_definition.even_permutation_combinations
    )

    cycle_combination_objs = []
    # TODO(pri 1/5 blocked on all parities): upper bound of LCM is math.lcm(*range(1, <max orbit cubie count> + 1))
    # TODO(pri 4/5): derive all lesser structures from max cubie count usage and fix only 1s
    # TODO(pri 5/5 blocked on all parities): share parity
    for used_cubie_counts in itertools.product(
        # when 0, the partition is all zeros which is disallowed later
        *(range(1, orbit.cubie_count + 1) for orbit in puzzle_orbit_definition.orbits)
    ):
        for all_partition_cubie_counts in itertools.product(
            *map(integer_partitions, used_cubie_counts),
        ):
            all_partition_cubie_counts = list(all_partition_cubie_counts)
            if any(
                len(partition_cubie_counts) > num_cycles
                for partition_cubie_counts in all_partition_cubie_counts
            ):
                continue
            for i, partition_cubie_counts in enumerate(all_partition_cubie_counts):
                all_partition_cubie_counts[i] += (0,) * (
                    num_cycles - len(partition_cubie_counts)
                )
            seen_cycle_cubie_counts = set()
            for all_permuted_partition_cubie_counts in itertools.product(
                *map(unique_permutations, all_partition_cubie_counts)
            ):
                all_cycle_cubie_counts = []
                continue_outer = False
                for cubie_counts in zip(*all_permuted_partition_cubie_counts):
                    # TODO(pri 5/5 blocked on derive all lesser): henry's faster impl
                    if all(
                        cubie_count == 0
                        or orbit.orientation_factor == OrientationFactor.One()
                        and cubie_count == 1
                        for orbit, cubie_count in zip(
                            puzzle_orbit_definition.orbits, cubie_counts
                        )
                    ):
                        continue_outer = True
                        break
                    all_cycle_cubie_counts.append(cubie_counts)
                if continue_outer:
                    continue
                all_cycle_cubie_counts = tuple(
                    sorted(all_cycle_cubie_counts, reverse=True)
                )
                if all_cycle_cubie_counts in seen_cycle_cubie_counts:
                    continue
                seen_cycle_cubie_counts.add(all_cycle_cubie_counts)
                for shared_cycle_combination in recursive_shared_cycle_combinations(
                    all_cycle_cubie_counts
                ):
                    orbits_can_share = [False] * len(puzzle_orbit_definition.orbits)
                    share_orbit_counts = [0] * len(puzzle_orbit_definition.orbits)
                    for cycle in shared_cycle_combination:
                        for i in range(len(puzzle_orbit_definition.orbits)):
                            orbits_can_share[i] |= (
                                cycle.share[i] is False
                                and 1 in cycle.partition_objs[i].partition
                            )
                            share_orbit_counts[i] += cycle.share[i]
                    if any(
                        share_orbit_count != 0 and not orbit_can_share
                        for share_orbit_count, orbit_can_share in zip(
                            share_orbit_counts, orbits_can_share
                        )
                    ):
                        continue
                    # just because we sort the parititons earlier doesnt mean the
                    # orders will be sorted
                    descending_order_cycle_combination = sorted(
                        shared_cycle_combination,
                        key=lambda cycle: (
                            cycle.order,
                            *map(
                                operator.attrgetter("partition"),
                                cycle.partition_objs,
                            ),
                        ),
                        reverse=True,
                    )
                    for i, start_cycle_to_permute in enumerate(
                        descending_order_cycle_combination
                    ):
                        if i == 0:
                            start_permuted_descending_order_cycle_combination = (
                                descending_order_cycle_combination
                            )
                        else:
                            # We only permute the cycles that have the same maximum
                            # order because the partition permutation for same order
                            # cycles matters for phase 2. Don't permute the rest
                            # because that logic is implemented in phase 3 (more
                            # efficient to do this in phase 3 vs here).
                            if (
                                start_cycle_to_permute.order
                                != descending_order_cycle_combination[0].order
                            ):
                                break
                            if all(
                                prev_cubie_partition == curr_cubie_partition
                                for prev_cubie_partition, curr_cubie_partition in zip(
                                    map(
                                        operator.attrgetter("partition"),
                                        descending_order_cycle_combination[
                                            i - 1
                                        ].partition_objs,
                                    ),
                                    map(
                                        operator.attrgetter("partition"),
                                        start_cycle_to_permute.partition_objs,
                                    ),
                                )
                            ):
                                continue
                            start_permuted_descending_order_cycle_combination = (
                                descending_order_cycle_combination.copy()
                            )
                            (
                                start_permuted_descending_order_cycle_combination[0],
                                start_permuted_descending_order_cycle_combination[i],
                            ) = (
                                start_permuted_descending_order_cycle_combination[i],
                                start_permuted_descending_order_cycle_combination[0],
                            )

                        for j in range(len(puzzle_orbit_definition.orbits)):
                            orbits_can_share[j] = False
                        all_share_orbit_cycle_candidates = [
                            [] for _ in range(len(puzzle_orbit_definition.orbits))
                        ]

                        order_product = 1
                        for j, cycle in enumerate(
                            start_permuted_descending_order_cycle_combination
                        ):
                            for k in range(len(puzzle_orbit_definition.orbits)):
                                if (
                                    orbits_can_share[k]
                                    and 1 in cycle.partition_objs[k].partition
                                ):
                                    all_share_orbit_cycle_candidates[k].append(j)
                                orbits_can_share[k] |= (
                                    1 in cycle.partition_objs[k].partition
                                )
                            order_product *= cycle.order

                        assert all(
                            share_orbit_count == 0
                            or len(share_orbit_cycle_candidates) != 0
                            for share_orbit_cycle_candidates, share_orbit_count in zip(
                                all_share_orbit_cycle_candidates, share_orbit_counts
                            )
                        )

                        share_orders = [
                            tuple(
                                tuple(
                                    j in share_orbit_indicies
                                    for share_orbit_indicies in all_share_orbit_indicies
                                )
                                for j in range(
                                    len(
                                        start_permuted_descending_order_cycle_combination
                                    )
                                )
                            )
                            for all_share_orbit_indicies in itertools.product(
                                # given a list "share_edge_candidates", what are all ways to
                                # pick "share_edge_count" numbers from the list
                                *(
                                    itertools.combinations(
                                        share_orbit_cycle_candidates,
                                        share_orbit_count,
                                    )
                                    for share_orbit_cycle_candidates, share_orbit_count in zip(
                                        all_share_orbit_cycle_candidates,
                                        share_orbit_counts,
                                    )
                                )
                            )
                        ]

                        # According to
                        # https://github.com/nestordemeure/paretoFront/blob/2aea69c371f70de4665f8abf24f6fda4ef0a8a70/src/pareto_front_implementation/pareto_front.rs#L265
                        # it is not worth removing redundant cycles
                        # intermediately
                        cycle_combination_objs.append(
                            CycleCombination(
                                used_cubie_counts=used_cubie_counts,
                                order_product=order_product,
                                share_orders=share_orders,
                                cycle_combination=start_permuted_descending_order_cycle_combination,
                            )
                        )
    return cycle_combination_objs


# do not flush cache it is used across used cubie counts
@functools.cache
def recursive_shared_cycle_combinations(all_cycle_cubie_counts):
    if len(all_cycle_cubie_counts) == 0:
        return ((),)
    return tuple(
        (shared_cycle,) + rest_combination
        for shared_cycle in highest_order_cycles_from_cubie_counts(
            all_cycle_cubie_counts[0]
        )
        for rest_combination in recursive_shared_cycle_combinations(
            all_cycle_cubie_counts[1:]
        )
    )


# TODO(pri 3/5 blocked on all parities): on bigger cubes where phase 2 is not applicable, do special
# optimizations that make this faster. only find the highest order
# product cycle dont care abt duplicates
@functools.cache
def highest_order_cycles_from_cubie_counts(cycle_cubie_counts):
    shared_cycles = []
    highest_order = 1
    share_states = []
    free_count = 0
    for i, cubie_count in enumerate(cycle_cubie_counts):
        if (
            cubie_count == 0
            # TODO(pri 5/5 blocked on deriving lesser): cubie_count == used_cubie_counts[i]
            or puzzle_orbit_definition_global.orbits[i].orientation_factor
            == OrientationFactor.One()
        ):
            share_states.append(ShareState.CANNOT_SHARE_ORIENTATION)
        elif cubie_count == 1:
            share_states.append(ShareState.MUST_SHARE_ORIENTATION)
        else:
            share_states.append(ShareState.FREE)
            free_count += 1
    for free_share in itertools.product(
        (False, True),
        repeat=free_count,
    ):
        share = []
        free_share_next_index = 0
        for share_state in share_states:
            match share_state:
                case ShareState.FREE:
                    share.append(free_share[free_share_next_index])
                    free_share_next_index += 1
                case ShareState.CANNOT_SHARE_ORIENTATION:
                    share.append(False)
                case ShareState.MUST_SHARE_ORIENTATION:
                    share.append(True)
        # TODO(pri 1/5): benchmark adding highest
        # number to stack first and then using a fibonacci heap
        all_reduced_integer_partitions = [
            reduced_integer_partitions(
                cycle_cubie_count,
                orbit,
                s,
            )
            for cycle_cubie_count, orbit, s in zip(
                cycle_cubie_counts,
                puzzle_orbit_definition_global.orbits,
                share,
            )
        ]

        rest_upper_bounds = []
        cycles = []
        partition_obj_path = [None] * len(all_reduced_integer_partitions)
        rest_upper_bound = 1

        for partition_obj in map(
            operator.itemgetter(0), all_reduced_integer_partitions
        ):
            rest_upper_bounds.append(rest_upper_bound)
            rest_upper_bound *= partition_obj.order

        stack = [(len(all_reduced_integer_partitions) - 1, 1, None)]
        while stack:
            i, running_order, partition_obj = stack.pop()
            if partition_obj is not None:
                partition_obj_path[i + 1] = partition_obj
            if i != -1:
                for partition_obj in all_reduced_integer_partitions[i]:
                    rest_upper_bound = running_order * partition_obj.order
                    if rest_upper_bound * rest_upper_bounds[i] < highest_order:
                        break
                    stack.append(
                        (
                            i - 1,
                            rest_upper_bound
                            // math.gcd(running_order, partition_obj.order),
                            partition_obj,
                        )
                    )
                continue
            if (
                sum(
                    (
                        sign(partition_obj.partition)
                        for partition_obj in partition_obj_path
                    )
                )
                % 2
                != 0
            ):
                continue
            if running_order > highest_order:
                cycles.clear()
            if running_order < highest_order:
                continue
            highest_order = running_order
            cycles.append(
                Cycle(
                    order=running_order,
                    share=share,
                    partition_objs=partition_obj_path.copy(),
                )
            )
        shared_cycles.extend(cycles)
    return shared_cycles


@functools.cache
def reduced_integer_partitions(cycle_cubie_count, orbit, s):
    partition_objs = []
    for partition in integer_partitions(cycle_cubie_count):
        if s:
            partition = (1,) + partition
        lcm = math.lcm(*partition)
        order = lcm

        always_orient = None
        critical_orient = None
        if isinstance(orbit.orientation_factor, OrientationFactor.GtOne):
            factor = orbit.orientation_factor.factor
            constraint = orbit.orientation_factor.constraint
            max_p_adic_valuation = -1
            for j, permutation_order in enumerate(partition):
                curr_p_adic_valuation = p_adic_valuation(
                    permutation_order,
                    factor,
                )
                if curr_p_adic_valuation > max_p_adic_valuation:
                    max_p_adic_valuation = curr_p_adic_valuation
                    critical_orient = [j]
                elif curr_p_adic_valuation == max_p_adic_valuation:
                    critical_orient.append(j)
                if permutation_order == 1:
                    if always_orient is None:
                        always_orient = [j]
                    else:
                        always_orient.append(j)
            match constraint:
                case OrientationSumConstraint.NONE:
                    if critical_orient is not None:
                        order *= factor
                case OrientationSumConstraint.ZERO:
                    orient_count = 0 if always_orient is None else len(always_orient)
                    critical_is_disjoint = critical_orient is not None and (
                        always_orient is None
                        or all(j not in always_orient for j in critical_orient)
                    )
                    if critical_is_disjoint:
                        orient_count += 1
                    ignore_critical_orient = orient_count == len(partition) and (
                        factor == 2
                        and orient_count % 2 == 1
                        or factor > 2
                        and orient_count == 1
                    )
                    if ignore_critical_orient:
                        if not critical_is_disjoint:
                            continue
                        assert len(critical_orient) == 1, critical_orient
                        critical_orient = None
                    # this is equivalent to len(partition) != 0 and the
                    # reasoning is an exercise left to the reader
                    elif orient_count != 0:
                        order *= factor
        partition_objs.append(
            CubiePartition(
                name=orbit.name,
                partition=partition,
                order=order,
                always_orient=always_orient,
                critical_orient=critical_orient,
            )
        )

    partition_objs.sort(reverse=True, key=operator.attrgetter("order"))
    dominated = [False] * len(partition_objs)
    reduced_partition_objs = []
    for i in range(len(partition_objs)):
        if dominated[i]:
            continue
        curr_partition_obj = partition_objs[i]
        reduced_partition_objs.append(curr_partition_obj)
        for j in range(i + 1, len(partition_objs)):
            if (
                curr_partition_obj.order % partition_objs[j].order == 0
                and curr_partition_obj.order != partition_objs[j].order
                and sign(curr_partition_obj.partition)
                == sign(partition_objs[j].partition)
            ):
                dominated[j] = True
    return reduced_partition_objs


def pareto_efficient_cycle_combinations(cycle_combination_objs):
    # This isnt the exact pareto efficient algorithm because I had trouble
    # getting it to work for some reason. The actual algorithm will be used in
    # the Rust verison of this code.
    cycle_combination_objs.sort(
        key=lambda cycle_combination_obj: (
            cycle_combination_obj.order_product,
            *map(operator.attrgetter("order"), cycle_combination_obj.cycle_combination),
        ),
        reverse=True,
    )
    pareto_points = []
    for maybe_redundant in cycle_combination_objs:
        if all(
            not cycle_combination_dominates(not_redundant, maybe_redundant)
            for not_redundant in pareto_points
        ):
            pareto_points.append(maybe_redundant)
    return pareto_points


def cycle_combination_objs_stats(cycle_combination_objs):
    a = collections.defaultdict(int)
    for cycle_combination_obj in cycle_combination_objs:
        a[
            tuple(
                zip(
                    map(
                        operator.attrgetter("order"),
                        cycle_combination_obj.cycle_combination,
                    )
                )
            )
        ] += len(cycle_combination_obj.share_orders)
    return dict(a)


def test():
    start = timeit.default_timer()
    cycle_combination_objs = pareto_efficient_cycle_combinations(
        all_cycle_combinations(
            PuzzleOrbitDefinition(
                orbits=[
                    Orbit(
                        name="edges",
                        cubie_count=12,
                        orientation_factor=OrientationFactor.GtOne(
                            factor=2,
                            constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                    Orbit(
                        name="corners",
                        cubie_count=8,
                        orientation_factor=OrientationFactor.GtOne(
                            factor=3,
                            constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                ],
                even_permutation_combinations=(("edges", "corners"),),
            ),
            2,
        )
    )

    stats = cycle_combination_objs_stats(cycle_combination_objs)
    assert stats == {
        ((90,), (90,)): 16,
        ((630,), (9,)): 4,
        ((180,), (30,)): 1,
        ((210,), (24,)): 1,
        ((126,), (36,)): 8,
        ((360,), (12,)): 4,
        ((720,), (2,)): 2,
    }, stats
    print("Passed test 1")

    cycle_combination_objs = pareto_efficient_cycle_combinations(
        all_cycle_combinations(
            PuzzleOrbitDefinition(
                orbits=[
                    Orbit(
                        name="edges",
                        cubie_count=12,
                        orientation_factor=OrientationFactor.GtOne(
                            factor=2,
                            constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                    Orbit(
                        name="corners",
                        cubie_count=8,
                        orientation_factor=OrientationFactor.GtOne(
                            factor=3,
                            constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                ],
                even_permutation_combinations=(("edges", "corners"),),
            ),
            3,
        )
    )

    stats = cycle_combination_objs_stats(cycle_combination_objs)
    assert stats == {
        ((90,), (90,), (6,)): 1,
        ((90,), (30,), (18,)): 1,
        ((30,), (30,), (30,)): 2,
        ((180,), (18,), (6,)): 2,
        ((126,), (12,), (12,)): 1,
        ((630,), (9,), (3,)): 1,
        ((210,), (9,), (9,)): 1,
        ((36,), (36,), (12,)): 1,
        ((126,), (36,), (3,)): 2,
        ((42,), (36,), (9,)): 2,
        ((360,), (6,), (6,)): 4,
        ((210,), (15,), (3,)): 1,
    }, stats
    print("Passed test 2")

    cycle_combination_objs = pareto_efficient_cycle_combinations(
        all_cycle_combinations(
            PuzzleOrbitDefinition(
                orbits=[
                    Orbit(
                        name="edges",
                        cubie_count=12,
                        orientation_factor=OrientationFactor.GtOne(
                            factor=2,
                            constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                    Orbit(
                        name="corners",
                        cubie_count=8,
                        orientation_factor=OrientationFactor.GtOne(
                            factor=3,
                            constraint=OrientationSumConstraint.ZERO,
                        ),
                    ),
                ],
                even_permutation_combinations=(("edges", "corners"),),
            ),
            4,
        )
    )

    stats = cycle_combination_objs_stats(cycle_combination_objs)
    assert stats == {
        ((90,), (24,), (6,), (6,)): 1,
        ((30,), (24,), (18,), (6,)): 1,
        ((126,), (12,), (6,), (6,)): 1,
        ((42,), (18,), (12,), (6,)): 1,
        ((30,), (12,), (12,), (12,)): 1,
        ((90,), (90,), (3,), (2,)): 1,
        ((90,), (30,), (9,), (2,)): 1,
        ((90,), (30,), (6,), (3,)): 8,
        ((90,), (18,), (10,), (3,)): 1,
        ((90,), (10,), (9,), (6,)): 1,
        ((30,), (30,), (18,), (3,)): 8,
        ((30,), (30,), (9,), (6,)): 8,
        ((30,), (18,), (10,), (9,)): 1,
        ((126,), (18,), (6,), (3,)): 1,
        ((90,), (36,), (6,), (2,)): 2,
        ((90,), (18,), (12,), (2,)): 2,
        ((90,), (12,), (12,), (3,)): 2,
        ((36,), (30,), (18,), (2,)): 2,
        ((36,), (30,), (12,), (3,)): 2,
        ((36,), (30,), (6,), (6,)): 16,
        ((18,), (18,), (12,), (10,)): 2,
        ((126,), (24,), (3,), (3,)): 1,
        ((42,), (24,), (9,), (3,)): 1,
        ((42,), (18,), (18,), (2,)): 5,
        ((60,), (45,), (3,), (3,)): 1,
        ((36,), (36,), (6,), (3,)): 4,
        ((210,), (6,), (6,), (3,)): 1,
        ((180,), (18,), (3,), (2,)): 2,
        ((180,), (12,), (3,), (3,)): 2,
        ((180,), (9,), (6,), (2,)): 2,
        ((630,), (3,), (3,), (3,)): 6,
        ((210,), (9,), (3,), (3,)): 7,
        ((360,), (6,), (3,), (2,)): 4,
        ((210,), (12,), (2,), (2,)): 1,
    }, stats
    end = timeit.default_timer()
    print(f"\nPassed tests in {end - start} seconds")
    print(recursive_shared_cycle_combinations.cache_info())
    print(highest_order_cycles_from_cubie_counts.cache_info())
    print(reduced_integer_partitions.cache_info())
    exit()


def main():
    a = timeit.default_timer()
    cycle_combinations = all_cycle_combinations(
        PuzzleOrbitDefinition(
            orbits=[
                Orbit(
                    name="edges",
                    cubie_count=12,
                    orientation_factor=OrientationFactor.GtOne(
                        factor=2,
                        constraint=OrientationSumConstraint.NONE,
                    ),
                ),
                Orbit(
                    name="corners",
                    cubie_count=8,
                    orientation_factor=OrientationFactor.GtOne(
                        factor=3,
                        constraint=OrientationSumConstraint.NONE,
                    ),
                ),
            ],
            # TODO(pri 5/5): all parities (sent in discord)
            even_permutation_combinations=(("edges", "corners"),),
            # even_permutation_combinations=(("centers", "corners"),),
            # even_permutation_combinations=(),
        ),
        2,
    )
    b = timeit.default_timer()
    print(b - a)
    print(recursive_shared_cycle_combinations.cache_info())
    print(highest_order_cycles_from_cubie_counts.cache_info())
    print(reduced_integer_partitions.cache_info())
    cycle_combinations = pareto_efficient_cycle_combinations(cycle_combinations)
    return cycle_combinations


if __name__ == "__main__":
    # test()
    cycle_combination_objs = main()
    with open("./output.py", "w") as f:
        f.write(
            f"Cycle = 1\nCycleCombination = 1\nCubiePartition = 1\n{
                cycle_combination_objs_stats(cycle_combination_objs)
            }\n{cycle_combination_objs}"
        )
