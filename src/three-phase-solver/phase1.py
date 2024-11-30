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

NO_ORIENT = 0
ORIENT = 1


Cycle = collections.namedtuple(
    "Cycle",
    [
        "order",
        "share",
        "edge_partition",
        "corner_partition",
        "always_orient_edges",
        "always_orient_corners",
        "critical_orient_edges",
        "critical_orient_corners",
    ],
)

CycleCombination = collections.namedtuple(
    "CycleCombination",
    [
        "used_edge_count",
        "used_corner_count",
        "order_product",
        # "share_order",
        "cycles",
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


@functools.cache
def shared_integer_partitions(n):
    """
    Adds a shared single permutation cycle to a partition to "share" it with
    another cycle. This works because {(1,) + i for i in integer_partitions(n)}
    == {i for i in integer_partitions(n + 1) if 1 in i}.
    """
    return {(1,) + i for i in integer_partitions(n)}


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
    return (-1) ** sum(k - 1 for k in partition)


def conditional_edge_factor(cond):
    return 2 if cond else 1


def conditional_corner_factor(cond):
    return 3 if cond else 1


def orientation_masks(masks, mask_length):
    return [] if mask_length == 0 else itertools.product(masks, repeat=mask_length)


def redundant_cycle_pairing(not_redundant, maybe_redundant):
    # A cycle pairing is redundant if it is pointless to include its order
    # compared to a non redundant cycle pairing. A lower order cycle pairing
    # can be perfectly described by a higher order cycle pairing.
    # e.g. (TGC: noting that the sets are sorted)
    # (60, 72) is redundant compared to (90, 90) because 60 < 90 and 72 <= 90
    # (45, 90) is redundant compared to (90, 90) because 45 < 90 and 72 <= 90
    # (18, 180) is redundant compared to (24, 210) because 18 < 24 and 180 <= 210
    # (12, 360) is NOT redundant compared to (24, 210) because 360 > 210
    redundant_order_pairing = True
    same_orders = True
    for maybe_redundant_cycle, not_redundant_cycle in zip(
        maybe_redundant.cycles,
        not_redundant.cycles,
    ):
        if maybe_redundant_cycle.order != not_redundant_cycle.order:
            same_orders = False
        if maybe_redundant_cycle.order > not_redundant_cycle.order:
            redundant_order_pairing = False
            break
    if not same_orders:
        return redundant_order_pairing
    else:
        return maybe_redundant == not_redundant
    # if any(
    #     maybe_redundant_cycle.edge_partition != not_redundant_cycle.edge_partition
    #     or maybe_redundant_cycle.corner_partition
    #     != not_redundant_cycle.corner_partition
    #     for maybe_redundant_cycle, not_redundant_cycle in zip(
    #         maybe_redundant.cycles,
    #         not_redundant.cycles,
    #     )
    # ):
    #     return False
    # # same_share_order = maybe_redundant.share_order == not_redundant.share_order
    # same_share_order =
    # return same_share_order


def all_cycle_combinations(num_cycles, edges, corners):
    """
    Finds all cycle structure pairings on the Rubik's cube.
    """
    cycle_combinations = []
    for used_edge_count, used_corner_count in itertools.product(
        range(edges + 1), range(corners + 1)
    ):
        for partition_edge_counts, partition_corner_counts in itertools.product(
            integer_partitions(used_edge_count),
            integer_partitions(used_corner_count),
        ):
            if (
                len(partition_edge_counts) > num_cycles
                or len(partition_corner_counts) > num_cycles
            ):
                continue
            partition_edge_counts += (0,) * (num_cycles - len(partition_edge_counts))
            partition_corner_counts += (0,) * (
                num_cycles - len(partition_corner_counts)
            )
            seen_cycle_cubie_counts = set()
            for (
                permuted_parition_edge_counts,
                permuted_partition_corner_counts,
            ) in itertools.product(
                # TODO: not efficient
                set(itertools.permutations(partition_edge_counts)),
                set(itertools.permutations(partition_corner_counts)),
            ):
                all_cycles_with_cubie_counts = []
                continue_outer = False
                for edge_count, corner_count in zip(
                    permuted_parition_edge_counts, permuted_partition_corner_counts
                ):
                    if edge_count == 0 and corner_count == 0:
                        continue_outer = True
                        break
                    all_cycles_with_cubie_counts.append((edge_count, corner_count))
                if continue_outer:
                    continue

                all_cycles_with_cubie_counts = tuple(
                    sorted(all_cycles_with_cubie_counts, reverse=True)
                )
                if all_cycles_with_cubie_counts in seen_cycle_cubie_counts:
                    continue

                seen_cycle_cubie_counts.add(all_cycles_with_cubie_counts)
                for shared_cycle_combination in recursive_shared_cycle_combinations(
                    all_cycles_with_cubie_counts
                ):
                    # just because we sort the parititons earlier doesnt mean the
                    # orders will be sorted
                    descending_order_cycle_combination = sorted(
                        shared_cycle_combination,
                        key=lambda cycle: (
                            cycle.order,
                            cycle.edge_partition,
                            cycle.corner_partition,
                        ),
                        reverse=True,
                    )

                    all_permuted_same_order_cycles = []
                    same_order_cycles = []
                    current_order = descending_order_cycle_combination[0].order
                    for i in range(len(descending_order_cycle_combination) + 1):
                        if (
                            i == len(descending_order_cycle_combination)
                            or descending_order_cycle_combination[i].order
                            != current_order
                        ):
                            permuted_same_order_cycles = []
                            # TODO: not efficient
                            seen_partitions = set()
                            for permuted_cycle in itertools.permutations(
                                same_order_cycles
                            ):
                                seen_key = tuple(
                                    (cycle.edge_partition, cycle.corner_partition)
                                    for cycle in permuted_cycle
                                )
                                if seen_key in seen_partitions:
                                    continue
                                seen_partitions.add(seen_key)
                                permuted_same_order_cycles.append(permuted_cycle)
                            all_permuted_same_order_cycles.append(
                                permuted_same_order_cycles
                            )
                            if i != len(descending_order_cycle_combination):
                                same_order_cycles = [
                                    descending_order_cycle_combination[i]
                                ]
                                current_order = descending_order_cycle_combination[
                                    i
                                ].order
                        else:
                            same_order_cycles.append(
                                descending_order_cycle_combination[i]
                            )

                    for descending_order_cycle_combination in map(
                        itertools.chain.from_iterable,
                        itertools.product(*all_permuted_same_order_cycles),
                    ):
                        descending_order_cycle_combination = list(
                            descending_order_cycle_combination
                        )
                        edge_can_share_exists = False
                        corner_can_share_exists = False
                        share_edge_count = 0
                        share_corner_count = 0
                        share_edge_candidates = []
                        share_corner_candidates = []
                        order_product = 1

                        for i, cycle in enumerate(descending_order_cycle_combination):
                            order_product *= cycle.order
                            if edge_can_share_exists and 1 in cycle.edge_partition:
                                share_edge_candidates.append(i)
                            if corner_can_share_exists and 1 in cycle.corner_partition:
                                share_corner_candidates.append(i)
                            edge_can_share_exists |= 1 in cycle.edge_partition
                            corner_can_share_exists |= 1 in cycle.corner_partition
                            share_edge_count += cycle.share[0]
                            share_corner_count += cycle.share[1]
                        # TODO: move this condition higher
                        if (
                            len(share_edge_candidates) == 0 and share_edge_count != 0
                        ) or (
                            len(share_corner_candidates) == 0
                            and share_corner_count != 0
                        ):
                            continue

                        # TODO: it might be possible that the tree search covers *every*
                        # possible way to distribute shares when the number of unshared
                        # cycles is greater than one. I am skeptical this is the case.
                        # consider 180/24 vs 126/36. 126/36
                        # is only found when 36 is first because the same partition
                        # produces 180/24 and 180 has the higher order as determined by
                        # >>> highest_order_cycles_from_cubie_counts(8, 5, False, False)
                        # [Cycle(order=180, share=(False, False), edge_partition=(1, 2, 5), corner_partition=(2, 3), always_orient_edges=[0], always_orient_corners=[], critical_orient_edges=[1], critical_orient_corners=[1])]
                        # >>> highest_order_cycles_from_cubie_counts(4, 3, True, False)
                        # [Cycle(order=24, share=(True, False), edge_partition=(1, 4), corner_partition=(1, 2), always_orient_edges=[0], always_orient_corners=[0], critical_orient_edges=[1], critical_orient_corners=[0, 1])]
                        # If I can show this tautology, then we can remove this
                        # part almost entirely which should significantly improve performance.
                        for (
                            share_edges_indicies,
                            share_corners_indicies,
                        ) in itertools.product(
                            # given a list "share_edge_candidates", what are all ways to
                            # pick "share_edge_count" numbers from the list
                            itertools.combinations(
                                share_edge_candidates, share_edge_count
                            ),
                            itertools.combinations(
                                share_corner_candidates, share_corner_count
                            ),
                        ):
                            cycle_combination = CycleCombination(
                                used_edge_count=used_edge_count,
                                used_corner_count=used_corner_count,
                                order_product=order_product,
                                cycles=tuple(
                                    cycle._replace(
                                        share=(
                                            i in share_edges_indicies,
                                            i in share_corners_indicies,
                                        )
                                    )
                                    for i, cycle in enumerate(
                                        descending_order_cycle_combination
                                    )
                                ),
                            )
                            cycle_combinations.append(cycle_combination)
                # TODO: is it worth removing redundant cycles intermediately?
                # this would require sorting by orders then re-sorting by order
                # product, so its performance vs memory
    return cycle_combinations


# do not flush cache it is used across used cubie counts
@functools.cache
def recursive_shared_cycle_combinations(cycle_cubie_counts):
    if len(cycle_cubie_counts) == 0:
        return ((),)
    share_mat = [(False, False)]
    # needed because when a cubie count is zero its partition is always the
    # empty tuple which logically cannot share a partition
    if cycle_cubie_counts[0][0] != 0:
        share_mat.append((True, False))
    if cycle_cubie_counts[0][1] != 0:
        share_mat.append((False, True))
    if cycle_cubie_counts[0][0] != 0 and cycle_cubie_counts[0][1] != 0:
        share_mat.append((True, True))
    return tuple(
        (shared_cycle,) + rest_combination
        for share in share_mat
        for shared_cycle in highest_order_cycles_from_cubie_counts(
            *cycle_cubie_counts[0], *share
        )
        for rest_combination in recursive_shared_cycle_combinations(
            cycle_cubie_counts[1:],
        )
    )


@functools.cache
def highest_order_cycles_from_cubie_counts(
    edge_count, corner_count, share_edge, share_corner
):
    """
    Given a set of edge and corner partitions, find the pairs of edge and corner
    partitions that yield the highest order cycle. Adapted from
    <https://gist.github.com/ScriptRaccoon/c12c4884c116dead62a15a3d09732d5d>
    with permission.
    """
    highest_order = 1
    cycles = []
    if edge_count == 0:
        assert corner_count != 0
        assert not share_edge
    if share_edge:
        edge_partitions = shared_integer_partitions(edge_count)
    else:
        edge_partitions = integer_partitions(edge_count)
    if corner_count == 0:
        assert not share_corner
    if share_corner:
        corner_partitions = shared_integer_partitions(corner_count)
    else:
        corner_partitions = integer_partitions(corner_count)
    for edge_partition, corner_partition in itertools.product(
        edge_partitions,
        corner_partitions,
    ):
        # Sign of partitions must be equal to ensure the cycle is possible.
        # Equivalent to checking for parity of edges and corners, as it is
        # impossible for just two edges to swap without two corners
        # swapping.
        if sign(corner_partition) != sign(edge_partition):
            continue

        always_orient_edges = []
        critical_orient_edges = None
        max_two_adic_valuation = -1
        for i, permutation_order in enumerate(edge_partition):
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
            curr_two_adic_valuation = p_adic_valuation(permutation_order, 2)
            if curr_two_adic_valuation > max_two_adic_valuation:
                max_two_adic_valuation = curr_two_adic_valuation
                critical_orient_edges = [i]
            elif curr_two_adic_valuation == max_two_adic_valuation:
                critical_orient_edges.append(i)
            # We force all order 1 permutation cycles to orient, otherwise
            # the cubie permutes in place (doesn't move). This voids the
            # necessity of that cycle and transposes the structure to
            # something else, constituting a logic error. Keep a mental note
            # that all one cycles MUST orient in a valid cycle structure.
            if permutation_order == 1:
                always_orient_edges.append(i)
        # Because the edge and corner orientation sum must be 0, we still
        # need to test whether the number of orientations of permutation
        # cycles is valid to guarantee that the cycle from the edge and
        # corner partitions is possible to form. Recall from sometime
        # earlier, we can treat orientations of permutation cycles as
        # orientations of cubies.
        orient_edge_count = len(always_orient_edges)
        # Remember that at least one critical cycle must orient. If this is
        # included in the always_orient_edges list, then we don't need to
        # orient any other critical cycles. However, if none of the critical
        # cycles are included in the always_orient_edges list, then we add
        # exactly one to the total orientation count for the oriented
        # critical cycle.
        critical_is_disjoint = critical_orient_edges is not None and all(
            i not in always_orient_edges for i in critical_orient_edges
        )
        if critical_is_disjoint:
            orient_edge_count += 1

        # TGC: it may be useful to rename this to 'non-critical' or similiar
        # they may still be 'valid' to use, just not using a critical flip
        invalid_orient_edge_count = (
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
            orient_edge_count == len(edge_partition)
            and
            # Same condition as explained some time earlier.
            orient_edge_count % 2 == 1
        )
        if invalid_orient_edge_count:
            # If always_orient_edges forces every permutation cycle to
            # orient, and there are an odd number of permutation cycles,
            # then this edge and partition pairing cannot form a cycle.
            # Example: (1, 1, 1) for edges
            if not critical_is_disjoint:
                continue
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
            assert len(critical_orient_edges) == 1, critical_orient_edges
            orient_edge_count -= 1
            critical_orient_edges = None

        # We do the same thing for corner partitions. It's conventional to
        # be DRY, but with how complicated the codebase already is it's most
        # readable this way.
        always_orient_corners = []
        critical_orient_corners = None
        max_three_adic_valuation = -1
        for i, permutation_order in enumerate(corner_partition):
            curr_three_adic_valuation = p_adic_valuation(permutation_order, 3)
            if curr_three_adic_valuation > max_three_adic_valuation:
                max_three_adic_valuation = curr_three_adic_valuation
                critical_orient_corners = [i]
            elif curr_three_adic_valuation == max_three_adic_valuation:
                critical_orient_corners.append(i)
            if permutation_order == 1:
                always_orient_corners.append(i)
        orient_corner_count = len(always_orient_corners)
        critical_is_disjoint = critical_orient_corners is not None and all(
            i not in always_orient_corners for i in critical_orient_corners
        )
        if critical_is_disjoint:
            orient_corner_count += 1
        invalid_orient_corner_count = (
            orient_corner_count == len(corner_partition) and orient_corner_count == 1
        )
        if invalid_orient_corner_count:
            if not critical_is_disjoint:
                continue
            assert len(critical_orient_corners) == 1, critical_orient_corners
            orient_corner_count -= 1
            critical_orient_corners = None

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
        # TODO: figure out similarities with this approach to
        # [Landau's function](https://en.wikipedia.org/wiki/Landau's_function)
        order = math.lcm(
            conditional_edge_factor(not invalid_orient_edge_count and edge_count != 0)
            * math.lcm(*edge_partition),
            conditional_corner_factor(
                not invalid_orient_corner_count and corner_count != 0
            )
            * math.lcm(*corner_partition),
        )
        # We only care about the highest order partition pairings.
        if order > highest_order:
            cycles = []
        if order < highest_order:
            continue
        highest_order = order
        cycles.append(
            Cycle(
                order=highest_order,
                share=(share_edge, share_corner),
                edge_partition=edge_partition,
                corner_partition=corner_partition,
                always_orient_edges=always_orient_edges,
                always_orient_corners=always_orient_corners,
                critical_orient_edges=critical_orient_edges,
                critical_orient_corners=critical_orient_corners,
            )
        )
    return cycles


def filter_redundant_cycle_combinations(cycle_combinations):
    """
    Removes all cycle pairings that fail the redundant_cycle_pairing test.
    """
    # TODO: Could you sort the cycles by the order of the first cycle, iterate through and only keep the ones with the highest second cycle, and then do the same thing for the second cycle only keeping the ones with the highest first cycle?
    cycle_combinations.sort(
        key=lambda cycle_combination: (
            cycle_combination.order_product,
            *map(operator.attrgetter("order"), cycle_combination.cycles),
        ),
        reverse=True,
    )
    filtered_cycle_combinations = []
    for maybe_redundant in cycle_combinations:
        if all(
            not redundant_cycle_pairing(not_redundant, maybe_redundant)
            for not_redundant in filtered_cycle_combinations
        ):
            filtered_cycle_combinations.append(maybe_redundant)
    return filtered_cycle_combinations


# TODO: change this to only find all possible corner structures because phase 2
# operates on only corners
def all_cycle_structures(cycle):
    """
    Given a cycle, find all possible cycle structures that can be formed from
    its edge and corner partitions (or permutation orders).

    A cycle structure is a tuple of integers that represents the number of
    cycles of each order that are present in the cycle. For example, the cycle
    structure (2, 1, 0, 0, 0, 0) represents two 2-cycles and a 3-cycle. Its
    encoding mirrors GAP's [CycleStructurePerm](https://docs.gap-system.org/doc/ref/chap42.html#X7944D1447804A69A).
    """
    cycle_structures = set()
    # The edge and corner partitions represent permutation orders, but this
    # obviously isn't the full story because cubies orient as well. We can
    # generalize this statement to say that permutations cycles also have an
    # orientation defined as the sum of each individual cubie's orientation that
    # make up the cycle. Since the orientation sum of every edge/corner must be
    # 0 modulo 2/3 (a basic truism), this implies the orientation sum of every
    # edge/corner permutation cycle must also be 0 modulo 2/3.

    # We consider all possible ways to orient edge and corner permutation
    # cycles, filtering out the invalid ones, and then compute the cycle
    # structure.
    for edge_orientation_mask in orientation_masks(
        [ORIENT, NO_ORIENT], len(cycle.edge_partition)
    ):
        if (
            # Cannot orient or "flip" an odd number of edge permutation cycles
            # (recall that we treat permutation cycles as cubies).
            edge_orientation_mask.count(ORIENT) % 2 == 1
            # explained later
            or any(
                edge_orientation_mask[i] == NO_ORIENT for i in cycle.always_orient_edges
            )
            or not (
                cycle.critical_orient_edges is None
                or any(
                    edge_orientation_mask[i] == ORIENT
                    for i in cycle.critical_orient_edges
                )
            )
        ):
            continue
        for corner_orientation_mask in orientation_masks(
            [ORIENT, NO_ORIENT], len(cycle.corner_partition)
        ):
            if (
                # Figuring out the amount of corner permutation cycles we are
                # allowed to orient is interesting. We can use a simple proof by
                # casing to show that we can orient any amount of cycles except
                # one.
                #
                # Case 1: No corner permutation cycles orient
                # The orientation sum of all corner permutation cycles is 0,
                # satisfying the 0 modulo 3 condition.
                # Case 2: One corner permutation cycle orients
                # The orientation of the oriented cycle isn't 0 by definition,
                # and the orientation sum of all other cycles is 0, contradicting
                # the 0 modulo 3 condition making this case invalid.
                # Case 3: Two corner permutation cycles orient
                # The orientation sum of the two oriented cycles can easily be
                # shown to be 0 modulo 3 if the first cycle's orientation is 1
                # and the second, 2.
                # Case 4: Three corner permutation cycles orient
                # The orientation sum of the three oriented cycles can easily be
                # shown to be 0 modulo 3 if all three cycles' orientations are 1.
                #
                # Any subsquent number of oriented cycles is actually just a
                # composition of the above cases. If the number is odd, we can
                # apply case 4 to make it even. If the number if even, we can
                # repeatedly apply case 3 for the remaining cycles.
                corner_orientation_mask.count(ORIENT) == 1
                # explained later
                or any(
                    corner_orientation_mask[i] == NO_ORIENT
                    for i in cycle.always_orient_corners
                )
                or not (
                    cycle.critical_orient_corners is None
                    or any(
                        corner_orientation_mask[i] == ORIENT
                        for i in cycle.critical_orient_corners
                    )
                )
            ):
                continue
            # We finally take into account orientation to find the true orders
            # of the cycle. It is then converted to the cycle structure
            # representation.
            edge_cycle_orders = [
                cycle_order
                * conditional_edge_factor(edge_orientation_mask[i] == ORIENT)
                for i, cycle_order in enumerate(cycle.edge_partition)
            ]
            corner_cycle_orders = [
                cycle_order
                * conditional_corner_factor(corner_orientation_mask[i] == ORIENT)
                for i, cycle_order in enumerate(cycle.corner_partition)
            ]

            cycle_structure = [0] * (max(*edge_cycle_orders, *corner_cycle_orders) - 1)
            for i, cycle_order in enumerate(edge_cycle_orders):
                # Sanity check, 1 cycles are unaffected cubies that should not
                # be present in the cycle structure, and why is explained later.
                assert cycle_order != 1
                # GAP's CycleStructurePerm is 2-indexed!
                cycle_structure[cycle_order - 2] += conditional_edge_factor(
                    edge_orientation_mask[i] == NO_ORIENT
                )

            for i, cycle_order in enumerate(corner_cycle_orders):
                assert cycle_order != 1
                cycle_structure[cycle_order - 2] += conditional_corner_factor(
                    corner_orientation_mask[i] == NO_ORIENT
                )
            cycle_structures.add(
                # (
                tuple(cycle_structure),
                #     cycle.edge_partition,
                #     edge_orientation_mask,
                #     cycle.corner_partition,
                #     corner_orientation_mask,
                # )
            )
    # Sanity check, guarantees in highest_order_cycles_from_cubie_counts ensure
    # that a cycle structure exists for every cycle. Else, the cycle is
    # impossible to form, and other possible high-order candidates from the same
    # partitions were never considered.
    assert cycle_structures != set(), cycle
    return frozenset(cycle_structures)


def main(num_cycles):
    from timeit import default_timer as timer

    a = timer()
    all_cycle_combinations_result = all_cycle_combinations(num_cycles, 12, 8)
    b = timer()
    print(b - a)
    print(recursive_shared_cycle_combinations.cache_info())
    filtered_cycle_combinations = filter_redundant_cycle_combinations(
        all_cycle_combinations_result
    )
    return filtered_cycle_combinations
    a = {}
    a = collections.defaultdict(lambda: 0)
    for cycle_combination in filtered_cycle_combinations:
        a[tuple(zip(map(operator.attrgetter("order"), cycle_combination.cycles)))] += 1
    return a


if __name__ == "__main__":
    with open("./output.py", "w") as f:
        f.write(f"Cycle = 1\nCycleCombination = 1\n{main(4)}")
