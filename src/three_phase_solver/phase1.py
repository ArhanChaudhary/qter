"""
Phase 1 of the three-phase solver.

This phase is responsible for finding pairs of commutative cycles on a Rubik's cube
that have high products of orders. The output of this phase is directly
used in phase 2.

Adapted with permission from ScriptRaccon's
<https://gist.github.com/ScriptRaccoon/c12c4884c116dead62a15a3d09732d5d>
"""

import collections
import itertools
import math
import operator
import functools
import timeit

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
        "cycles",
    ],
)


Cycle = collections.namedtuple(
    "Cycle",
    [
        "order",
        "share",
        "cubie_partitions",
    ],
)


CubiePartition = collections.namedtuple(
    "CubiePartition",
    [
        "name",
        "partition",
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


def conditional_edge_factor(cond):
    return 2 if cond else 1


def conditional_corner_factor(cond):
    return 3 if cond else 1


def orientation_masks(masks, mask_length):
    return [] if mask_length == 0 else itertools.product(masks, repeat=mask_length)


def cycle_combination_dominates(this, other):
    # A modification of the weakly dominates condition in the pareto efficient
    # algorithm
    different_orders = False
    same_cycle = True
    for this_cycle, other_cycle in zip(this.cycles, other.cycles):
        if this_cycle.order < other_cycle.order:
            return False
        elif not different_orders:
            different_orders |= this_cycle.order > other_cycle.order
            same_cycle &= this_cycle.share == other_cycle.share and all(
                this_cubie_partition.partition == other_cubie_partition.partition
                for this_cubie_partition, other_cubie_partition in zip(
                    this_cycle.cubie_partitions, other_cycle.cubie_partitions
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

    cycle_combinations = []
    for used_cubie_counts in itertools.product(
        *(range(orbit.cubie_count + 1) for orbit in puzzle_orbit_definition.orbits)
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
                    # TODO: when orientation factor is 1, disallow stuff like
                    # CubiePartition(
                    #     name="wings",
                    #     partition=(1,),
                    #     always_orient=[],
                    #     critical_orient=None,
                    # ),
                    if all(cubie_count == 0 for cubie_count in cubie_counts):
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
                    share_orbits_exists = [False] * len(puzzle_orbit_definition.orbits)
                    all_orbits_can_share = True
                    for cycle in shared_cycle_combination:
                        all_orbits_can_share = True
                        for i in range(len(puzzle_orbit_definition.orbits)):
                            if not orbits_can_share[i]:
                                all_orbits_can_share = False
                                share_orbits_exists[i] |= cycle.share[i]
                                orbits_can_share[i] |= (
                                    cycle.share[i] is False
                                    and 1 in cycle.cubie_partitions[i].partition
                                )
                        if all_orbits_can_share:
                            break
                    if not all_orbits_can_share and any(
                        share_orbits_exists[i] and not orbits_can_share[i]
                        for i in range(len(puzzle_orbit_definition.orbits))
                    ):
                        continue
                    # just because we sort the parititons earlier doesnt mean the
                    # orders will be sorted
                    descending_order_cycle_combination = sorted(
                        shared_cycle_combination,
                        key=lambda cycle: (cycle.order, *cycle.cubie_partitions),
                        reverse=True,
                    )
                    cycle_combinations.append(
                        CycleCombination(
                            used_cubie_counts=used_cubie_counts,
                            order_product=math.prod(
                                map(
                                    operator.attrgetter("order"),
                                    descending_order_cycle_combination,
                                )
                            ),
                            cycles=descending_order_cycle_combination,
                        )
                    )
                    continue
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
                            if (
                                descending_order_cycle_combination[i - 1].edge_partition
                                == start_cycle_to_permute.edge_partition
                                and descending_order_cycle_combination[
                                    i - 1
                                ].corner_partition
                                == start_cycle_to_permute.corner_partition
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

                        edge_can_share_exists = False
                        corner_can_share_exists = False
                        share_edge_count = 0
                        share_corner_count = 0
                        share_edge_cycle_candidates = []
                        share_corner_cycle_candidates = []
                        order_product = 1

                        for j, cycle in enumerate(
                            start_permuted_descending_order_cycle_combination
                        ):
                            if edge_can_share_exists and 1 in cycle.edge_partition:
                                share_edge_cycle_candidates.append(j)
                            if corner_can_share_exists and 1 in cycle.corner_partition:
                                share_corner_cycle_candidates.append(j)

                            edge_can_share_exists |= 1 in cycle.edge_partition
                            corner_can_share_exists |= 1 in cycle.corner_partition
                            share_edge_count += cycle.share[0]
                            share_corner_count += cycle.share[1]
                            order_product *= cycle.order

                        assert (
                            share_edge_count == 0
                            or len(share_edge_cycle_candidates) != 0
                        ) and (
                            share_corner_count == 0
                            or len(share_corner_cycle_candidates) != 0
                        )
                        for (
                            share_edge_indicies,
                            share_corner_indicies,
                        ) in itertools.product(
                            # given a list "share_edge_candidates", what are all ways to
                            # pick "share_edge_count" numbers from the list
                            itertools.combinations(
                                share_edge_cycle_candidates, share_edge_count
                            ),
                            itertools.combinations(
                                share_corner_cycle_candidates, share_corner_count
                            ),
                        ):
                            # According to
                            # https://github.com/nestordemeure/paretoFront/blob/2aea69c371f70de4665f8abf24f6fda4ef0a8a70/src/pareto_front_implementation/pareto_front.rs#L265
                            # it is not worth removing redundant cycles
                            # intermediately
                            cycle_combination = CycleCombination(
                                used_cubie_counts=used_cubie_counts,
                                order_product=order_product,
                                cycles=tuple(
                                    cycle._replace(
                                        share=(
                                            j in share_edge_indicies,
                                            j in share_corner_indicies,
                                        )
                                    )
                                    for j, cycle in enumerate(
                                        start_permuted_descending_order_cycle_combination
                                    )
                                ),
                            )
                            cycle_combinations.append(cycle_combination)
    return cycle_combinations


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


@functools.cache
def reduced_integer_partitions(cycle_cubie_count, s):
    partitions = [
        # This works because {(1,) + i for i in integer_partitions(n)}
        # == {i for i in integer_partitions(n + 1) if 1 in i}.
        (math.lcm(*partition), (1,) + partition if s else partition)
        for partition in integer_partitions(cycle_cubie_count)
    ]
    partitions.sort(reverse=True, key=operator.itemgetter(0))
    dominated = [False] * len(partitions)
    reduced_partitions = []
    for i in range(len(partitions)):
        if dominated[i]:
            continue
        current = partitions[i]
        reduced_partitions.append(current)
        for j in range(i + 1, len(partitions)):
            # TODO: this does not account for the orientation factor. Using 2
            # corners: parition (1,1) admits a 3 cycle but its LCM is 1.
            # Partition (2) admits a 2 cycle
            if (
                current[0] % partitions[j][0] == 0
                and current[0] != partitions[j][0]
                # TODO: this does not account for when there is no parity
                and sign(current[1]) == sign(partitions[j][1])
            ):
                dominated[j] = True
    return reduced_partitions


# TODO: is cache necessary?
@functools.cache
def highest_order_cycles_from_cubie_counts(cycle_cubie_counts):
    """
    Given a set of edge and corner partitions, find the pairs of edge and corner
    partitions that yield the highest order cycle. Adapted from
    <https://gist.github.com/ScriptRaccoon/c12c4884c116dead62a15a3d09732d5d>
    with permission.
    """
    ret = []
    # TODO: make this a number and the array its bits. get rid of the second
    # pass as well
    for partial_share in itertools.product(
        (False, True),
        repeat=sum(1 for cubie_count in cycle_cubie_counts if cubie_count != 0),
    ):
        share = []
        i = 0
        for cubie_count in cycle_cubie_counts:
            if cubie_count == 0:
                share.append(False)
            else:
                share.append(partial_share[i])
                i += 1
        assert i == len(partial_share)
        # TODO: should highest_order be defined in the outer loop?
        highest_order = 1
        cycles = []
        for lcm_and_cubie_partitions in itertools.product(
            *(
                reduced_integer_partitions(cycle_cubie_count, s)
                for s, cycle_cubie_count in zip(share, cycle_cubie_counts)
            )
        ):
            if any(
                # TODO: all ways to handle parities
                sum(
                    sign(lcm_and_cubie_partitions[i][1])
                    for i in even_permutation_combination_indicies
                )
                % 2
                != 0
                for even_permutation_combination_indicies in even_permutation_combinations_indicies_global
            ):
                continue

            continue_outer = False
            cubie_partitions_obj = []
            running_order = 1
            for i, lcm_and_cubie_partitions in enumerate(lcm_and_cubie_partitions):
                lcm, cubie_partition = lcm_and_cubie_partitions
                always_orient = []
                critical_orient = None
                if puzzle_orbit_definition_global.orbits[i].orientation_factor != 1:
                    max_p_adic_valuation = -1
                    for j, permutation_order in enumerate(cubie_partition):
                        # Given our partition, we want to figure out which permutation
                        # cycles must orient to ensure the order remains the same if
                        # every permuation cycle were to orient. Since the order
                        # calculation is lcm(2a, 2b, ... 2z), the cycle order(s) with
                        # the most 2s in its prime factorization will be the leading
                        # coefficient for the LCM and therefore must orient. It just so
                        # happens that this type of computation is equivalent to finding
                        # the 2-adic valuation of each permutation cycle order, and I
                        # have no idea why. Maybe there's a more fundamental reason.

                        # We define the cycles that must orient as "critical" because
                        # at least one of them must orient to ensure the order remains
                        # the same.
                        curr_p_adic_valuation = p_adic_valuation(
                            permutation_order,
                            puzzle_orbit_definition_global.orbits[i].orientation_factor,
                        )
                        if curr_p_adic_valuation > max_p_adic_valuation:
                            max_p_adic_valuation = curr_p_adic_valuation
                            critical_orient = [j]
                        elif curr_p_adic_valuation == max_p_adic_valuation:
                            critical_orient.append(j)
                        # We force all order 1 permutation cycles to orient, otherwise
                        # the cubie permutes in place (doesn't move). This voids the
                        # necessity of that cycle and transposes the structure to
                        # something else, constituting a logic error. Keep a mental note
                        # that all one cycles MUST orient in a valid cycle structure.
                        if permutation_order == 1:
                            always_orient.append(j)
                    # Because the edge and corner orientation sum must be 0, we still
                    # need to test whether the number of orientations of permutation
                    # cycles is valid to guarantee that the cycle from the edge and
                    # corner partitions is possible to form. Recall from sometime
                    # earlier, we can treat orientations of permutation cycles as
                    # orientations of cubies.
                    orient_count = len(always_orient)
                    # Remember that at least one critical cycle must orient. If this is
                    # included in the always_orient_edges list, then we don't need to
                    # orient any other critical cycles. However, if none of the critical
                    # cycles are included in the always_orient_edges list, then we add
                    # exactly one to the total orientation count for the oriented
                    # critical cycle.
                    critical_is_disjoint = critical_orient is not None and all(
                        j not in always_orient for j in critical_orient
                    )
                    if critical_is_disjoint:
                        orient_count += 1

                    ignore_critical_orient = (
                        # Before determining if a cycle is possible, first ensure that
                        # every permutation cycle must orient.
                        # If orientation is even, we're fine. If it's not, but there is an
                        # extra cycle, We'll be able to orient it to make total orientation
                        # even. The issue comes when we've already used all cycles and still
                        # have odd orientation. This means we have to unorient a critical
                        # cycle to make orientation even. Example: Given the partition
                        # (1, 1, 2, 2) for edges all the ones must orient and at least one
                        # two must orient. Although the total number of cycle orientations
                        # is odd, the partition is still possible if everything orients.
                        # This is not the case with (1, 1, 2).
                        orient_count == len(cubie_partition)
                        and (
                            puzzle_orbit_definition_global.orbits[i].orientation_factor
                            == 2
                            and orient_count % 2 == 1
                            or puzzle_orbit_definition_global.orbits[
                                i
                            ].orientation_factor
                            > 2
                            and orient_count == 1
                        )
                    )
                    if ignore_critical_orient:
                        # If always_orient_edges forces every permutation cycle to
                        # orient, and there are an odd number of permutation cycles,
                        # then this edge and partition pairing cannot form a cycle.
                        # Example: (1, 1, 1) for edges
                        if not critical_is_disjoint:
                            continue_outer = True
                            break
                        # If always_orient_edges forces every permutation cycle to
                        # orient except for a critical permutation cycle, we
                        # nullify the critical cycle, accepting the consequence of a
                        # lower highest possible order, to make the cycle possible.

                        # Now, there may be mutiple permutation cycles that have the
                        # same maximum 2-adic valuation, meaning that the order wouldn't
                        # actually change if the critical cycle were not to orient.
                        # Strangely, during testing, I found out that this never was
                        # the case, and I don't need to worry about it. The following
                        # assertion never fails, implying that the critical cycle is the
                        # only cycle with the maximum 2-adic valuation.
                        # TGC: This assertion never fails because this is the case where we
                        # have odd orientation and no extra cycles to use to fix it, so we
                        # must fix by unorienting a critical cycle.
                        assert len(critical_orient) == 1, critical_orient
                        orient_count -= 1
                        critical_orient = None
                cubie_partitions_obj.append(
                    CubiePartition(
                        name=puzzle_orbit_definition_global.orbits[i].name,
                        partition=cubie_partition,
                        always_orient=always_orient,
                        critical_orient=critical_orient,
                    )
                )
                # TODO: maybe replace with cubie_count == 0
                if cubie_partition != ():
                    cycle_order = lcm
                    if (
                        puzzle_orbit_definition_global.orbits[i].orientation_factor != 1
                        and not ignore_critical_orient
                    ):
                        cycle_order *= puzzle_orbit_definition_global.orbits[
                            i
                        ].orientation_factor
                    # TODO: aggressively enforce coprime? if not then continue_outer
                    running_order = math.lcm(
                        running_order,
                        cycle_order,
                    )
            if continue_outer:
                continue

            # Let's consider the case for edges to simplify the explanation.
            # We have a bunch of permutation cycle orders, which I will name
            # letters a through z, and some of these orient, meaning their
            # orders double from flipping. So, lcm(2a, 2b, ... 2z) is the edge
            # order. We can extract out the 2s to get 2 * lcm(a, b, ... z), and
            # this is almost the full story. Remember in the case of an invalid
            # cycle orientation count, we nullify the critical cycle to make the
            # cycle possible. The critical cycle was defined as the cycle with
            # the most 2s in its prime factorization, so if we remove a factor
            # of two from the critical cycle then we must also remove a factor
            # of two from the full edge order. Conveniently, this can be
            # made simple by changing the leading 2 to a 1 in this case, to get
            # 1 * lcm(a, b, ... z). The corners follow the same logic.
            # NOTE: based on https://math.stackexchange.com/a/3029900, it might be
            # worth removing orders that divides one another because the LCM is
            # guaranteed to not be greater. For now, there does not seem to be much
            # use in doing this.
            if running_order > highest_order:
                cycles = []
            if running_order < highest_order:
                continue
            # TODO: on bigger cubes where phase 2 is not applicable, do special
            # optimizations that make this faster. 1. only care about creating
            # the highest order from prime powers 2. only find the highest order
            # product cycle
            cycles.append(
                Cycle(
                    order=running_order,
                    share=share,
                    cubie_partitions=cubie_partitions_obj,
                )
            )
            highest_order = running_order
        ret.extend(cycles)
    return ret


def pareto_efficient_cycle_combinations(cycle_combinations):
    # This isnt the exact pareto efficient algorithm because I had trouble
    # getting it to work for some reason. The actual algorithm will be used in
    # the Rust verison of this code.
    cycle_combinations.sort(
        key=lambda cycle_combination: (
            cycle_combination.order_product,
            *map(operator.attrgetter("order"), cycle_combination.cycles),
        ),
        reverse=True,
    )
    pareto_points = []
    for maybe_redundant in cycle_combinations:
        if all(
            not cycle_combination_dominates(not_redundant, maybe_redundant)
            for not_redundant in pareto_points
        ):
            pareto_points.append(maybe_redundant)
    return pareto_points


def main():
    a = timeit.default_timer()
    cycle_combinations = all_cycle_combinations(
        PuzzleOrbitDefinition(
            orbits=[
                # TODO: Orbit(name="corners", cubie_count=8, orientation_factor=3, orientation_sum_constraint=ZERO),
                # TODO: Orbit(name="corners", cubie_count=8, orientation_factor=3, orientation_sum_constraint=ANYTHING),
                # Orbit(name="edges", cubie_count=12, orientation_factor=2),
                # Orbit(name="corners", cubie_count=8, orientation_factor=3),
                Orbit(name="centers", cubie_count=24, orientation_factor=1),
                Orbit(name="wings", cubie_count=24, orientation_factor=1),
                Orbit(name="corners", cubie_count=8, orientation_factor=3),
            ],
            # TODO: all parities (sent in discord)
            # even_permutation_combinations=(("edges", "corners"),),
            even_permutation_combinations=(("centers", "corners"),),
        ),
        2,
    )
    b = timeit.default_timer()
    print(b - a)
    print(recursive_shared_cycle_combinations.cache_info())
    cycle_combinations.sort(
        key=lambda cycle_combination: (
            cycle_combination.order_product,
            *map(operator.attrgetter("order"), cycle_combination.cycles),
        ),
        reverse=True,
    )
    cycle_combinations = pareto_efficient_cycle_combinations(cycle_combinations)
    return cycle_combinations
    a = {}
    a = collections.defaultdict(int)
    for cycle_combination in cycle_combinations:
        a[tuple(zip(map(operator.attrgetter("order"), cycle_combination.cycles)))] += 1
    return dict(a)


if __name__ == "__main__":
    with open("./output.py", "w") as f:
        f.write(f"Cycle = 1\nCycleCombination = 1\nCubiePartition = 1\n{main()}")
