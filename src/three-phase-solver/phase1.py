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

EDGES = 12
CORNERS = 8
NO_ORIENT = 0
ORIENT = 1


@functools.lru_cache
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
    answer = {(n,)}
    for x in range(1, n):
        for y in integer_partitions(n - x):
            answer.add(tuple(sorted((x,) + y)))
    return answer


def share_partitions(partitions):
    """
    Adds a shared single permutation cycle to a partition to "share" it with
    another cycle. This works because {(1,) + i for i in partitions(n)} == {i
    for i in partitions(n + 1) if 1 in i}.
    """
    return {(1,) + i for i in partitions}


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
    # e.g.
    # (60, 72) is redundant compared to (90, 90)
    # (45, 90) is redundant compared to (90, 90)
    # (90, 45) is redundant compared to (90, 90)
    # (18, 180) is redundant compared to (24, 210)
    # (12, 360) is NOT redundant compared to (24, 210)
    redundant_order_pairing = True
    same_orders = True
    for maybe_redundant_cycle, not_redundant_cycle in zip(
        maybe_redundant["cycles"],
        not_redundant["cycles"],
    ):
        if maybe_redundant_cycle["order"] != not_redundant_cycle["order"]:
            same_orders = False
        if maybe_redundant_cycle["order"] > not_redundant_cycle["order"]:
            redundant_order_pairing = False
            break
    if redundant_order_pairing and not same_orders:
        return True

    # A cycle pairing is redundant if first_cycle and second_cycle share the
    # same edge and corner partitions as a non redundant cycle pairing,
    # optionally swapped. !!! We also need to check if they both share the same
    # cubies because those differentiate two cycle structures.
    maybe_redundant_counter = collections.Counter(
        (
            maybe_redundant_cycle["edge_partition"],
            maybe_redundant_cycle["corner_partition"],
        )
        for maybe_redundant_cycle in maybe_redundant["cycles"]
    )
    not_redundant_counter = collections.Counter(
        (not_redundant_cycle["edge_partition"], not_redundant_cycle["corner_partition"])
        for not_redundant_cycle in not_redundant["cycles"]
    )
    redundant_partition_pairing = maybe_redundant_counter == not_redundant_counter
    return redundant_partition_pairing


def all_cycle_pairings(num_cycles=2):
    """
    Finds all cycle structure pairings on the Rubik's cube.
    """
    cycle_combinations = []
    # Even though the end goal is to find the most optimal cycle pairings, there
    # exists optimal pairings whose generators leave some cubies intact. We
    # perform the described computation for all edge and corner counts.
    for used_edge_count, used_corner_count in itertools.product(
        range(2, EDGES + 1), range(2, CORNERS + 1)
    ):
        # - 1 for sharing an edge and corner
        for parition_edge_counts, partition_corner_counts in itertools.product(
            integer_partitions(used_edge_count - 1),
            integer_partitions(used_corner_count - 1),
        ):
            if (
                len(parition_edge_counts) != num_cycles
                or len(partition_corner_counts) != num_cycles
            ):
                continue
            # TODO: this is GROSSLY inefficient in that it overcounts the same
            # thing num_cycles! times, can be made more efficient with some more
            # work
            for (
                parition_edge_counts,
                partition_corner_counts,
            ) in itertools.product(
                set(itertools.permutations(parition_edge_counts)),
                set(itertools.permutations(partition_corner_counts)),
            ):
                cycle_generators = []
                for edge_count, corner_count in zip(
                    parition_edge_counts, partition_corner_counts
                ):
                    cycle_generators.append(
                        highest_order_cycles_from_partitions(
                            share_partitions(integer_partitions(edge_count)),
                            share_partitions(integer_partitions(corner_count)),
                        )
                    )
                for cycle_combination in itertools.product(*cycle_generators):
                    cycle_combinations.append(
                        {
                            "used_edge_count": used_edge_count,
                            "used_corner_count": used_corner_count,
                            "cycles": list(
                                sorted(
                                    cycle_combination,
                                    key=operator.itemgetter("order"),
                                    reverse=True,
                                )
                            ),
                            "order_product": math.prod(
                                cycle["order"] for cycle in cycle_combination
                            ),
                        }
                    )
    return cycle_combinations


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
        [ORIENT, NO_ORIENT], len(cycle["edge_partition"])
    ):
        if (
            # Cannot orient or "flip" an odd number of edge permutation cycles
            # (recall that we treat permutation cycles as cubies).
            edge_orientation_mask.count(ORIENT) % 2 == 1
            # explained later
            or any(
                edge_orientation_mask[i] == NO_ORIENT
                for i in cycle["always_orient_edges"]
            )
            or not (
                cycle["critical_orient_edges"] is None
                or any(
                    edge_orientation_mask[i] == ORIENT
                    for i in cycle["critical_orient_edges"]
                )
            )
        ):
            continue
        for corner_orientation_mask in orientation_masks(
            [ORIENT, NO_ORIENT], len(cycle["corner_partition"])
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
                    for i in cycle["always_orient_corners"]
                )
                or not (
                    cycle["critical_orient_corners"] is None
                    or any(
                        corner_orientation_mask[i] == ORIENT
                        for i in cycle["critical_orient_corners"]
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
                for i, cycle_order in enumerate(cycle["edge_partition"])
            ]
            corner_cycle_orders = [
                cycle_order
                * conditional_corner_factor(corner_orientation_mask[i] == ORIENT)
                for i, cycle_order in enumerate(cycle["corner_partition"])
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
                #     cycle["edge_partition"],
                #     edge_orientation_mask,
                #     cycle["corner_partition"],
                #     corner_orientation_mask,
                # )
            )
    # Sanity check, guarantees in highest_order_cycles_from_partitions ensure
    # that a cycle structure exists for every cycle. Else, the cycle is
    # impossible to form, and other possible high-order candidates from the same
    # partitions were never considered.
    assert cycle_structures != set(), cycle
    return frozenset(cycle_structures)


def highest_order_cycles_from_partitions(edge_partitions, corner_partitions):
    """
    Given a set of edge and corner partitions, find the pairs of edge and corner
    partitions that yield the highest order cycle. Adapted from
    <https://gist.github.com/ScriptRaccoon/c12c4884c116dead62a15a3d09732d5d>
    with permission.
    """
    highest_order = 1
    cycles = []
    for edge_partition in edge_partitions:
        for corner_partition in corner_partitions:
            # Sign of partitions must be equal to ensure the cycle is possible.
            # Equivalent to checking for parity of edges and corners, as it is
            # impossible for just two edges to swap without two corners
            # swapping.
            if sign(corner_partition) != sign(edge_partition):
                continue

            always_orient_edges = []
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
            invalid_orient_edge_count = (
                # Before determining if a cycle is possible, first ensure that
                # every permutation cycle must orient.
                # TODO: I'm not entirely sure of this condition's correctness,
                # but I can provide an example. Given the partition
                # (1, 1, 2, 2) for edges all the ones must orient and at
                # least one two must orient. Although the total number of cycle
                # orientations is odd, the partition is still possible if
                # everything orients. This is not the case with (1, 1, 2).
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
                # TODO: Figure out why this assertion never fails.
                assert len(critical_orient_edges) == 1, critical_orient_edges
                orient_edge_count -= 1
                critical_orient_edges = None

            # We do the same thing for corner partitions. It's conventional to
            # be DRY, but with how complicated the codebase already is it's most
            # readable this way.
            always_orient_corners = []
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
                orient_corner_count == len(corner_partition)
                and orient_corner_count == 1
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
                conditional_edge_factor(not invalid_orient_edge_count)
                * math.lcm(*edge_partition),
                conditional_corner_factor(not invalid_orient_corner_count)
                * math.lcm(*corner_partition),
            )
            # We only care about the highest order partition pairings.
            if order > highest_order:
                cycles = []
            if order < highest_order:
                continue
            highest_order = order
            cycle = {
                "order": highest_order,
                "edge_partition": edge_partition,
                "corner_partition": corner_partition,
                "always_orient_edges": always_orient_edges,
                "always_orient_corners": always_orient_corners,
                "critical_orient_edges": critical_orient_edges,
                "critical_orient_corners": critical_orient_corners,
            }
            cycle["structures"] = all_cycle_structures(cycle)
            cycles.append(cycle)
    return cycles


def filter_redundant_cycle_pairings(cycle_pairings):
    """
    Removes all cycle pairings that fail the redundant_cycle_pairing test. Also
    sorts the cycle pairings by order_product in descending order.
    """
    filtered_cycle_pairings = []
    for maybe_redundant in sorted(
        cycle_pairings,
        key=operator.itemgetter("order_product"),
        reverse=True,
    ):
        if all(
            not redundant_cycle_pairing(not_redundant, maybe_redundant)
            for not_redundant in filtered_cycle_pairings
        ):
            filtered_cycle_pairings.append(maybe_redundant)
    return filtered_cycle_pairings


def group_cycle_pairings(cycle_pairings):
    """
    Organizes the final structure and removes redundant cycle pairings that are
    phase 3 specific.
    """
    # It's a bit difficult to understand how this works from the code itself,
    # so I'll give a high-level overview of the problem this function solves.
    # Suppose we have two cycle pairings that are the same except the first
    # shares an edge and the second does not. If we advance the cycle pairings
    # to phase 3 where we stabilize the (equivalent) output from phase 2, add
    # the generator sharing the edge for the first cycle pairing, notice that
    # the elements of the second cycle pairing's stabilizer are a subset of the
    # elements of the first cycle pairing's stabilizer. That is, the generator
    # doesn't *have* to be used to produce elements of a group. We don't want
    # to double count, so this function just considers the cycle pairing that
    # share the most cubies.
    cycle_to_share_info = {}
    for cycle_pairing in cycle_pairings:
        key = (
            cycle_pairing["first_cycle"]["order"],
            cycle_pairing["first_cycle"]["structures"],
            cycle_pairing["second_cycle"]["order"],
            # cycle_pairing["second_cycle"]["structures"],
        )
        value = (cycle_pairing["share_edge"], cycle_pairing["share_corner"])
        # For every cycle pairing, we keep track of which has the most number
        # of shared cubies. The > operator for tuples helps us achieve this.
        if key not in cycle_to_share_info or value > cycle_to_share_info[key]:
            cycle_to_share_info[key] = value
        # If the cycle pairings have the same order we must re-run the
        # computation for the second cycle pairing as well.
        if (
            cycle_pairing["first_cycle"]["order"]
            == cycle_pairing["second_cycle"]["order"]
        ):
            key = (
                cycle_pairing["second_cycle"]["order"],
                cycle_pairing["second_cycle"]["structures"],
                cycle_pairing["first_cycle"]["order"],
                # cycle_pairing["first_cycle"]["structures"],
            )
            if key not in cycle_to_share_info or value > cycle_to_share_info[key]:
                cycle_to_share_info[key] = value
    grouped_cycle_pairings = []
    for key, value in cycle_to_share_info.items():
        grouped_cycle_pairings.append(
            {
                "share_edge": value[0],
                "share_corner": value[1],
                "first_cycle_order": key[0],
                "first_cycle_structures": key[1],
                "second_cycle_order": key[2],
                # "second_cycle_structures": key[3],
            }
        )
    # for cycle_pairing in cycle_pairings:
    #     grouped_cycle_pairings.append(
    #         {
    #             "share_edge": cycle_pairing["share_edge"],
    #             "share_corner": cycle_pairing["share_corner"],
    #             "first_cycle_order": cycle_pairing["first_cycle"]["order"],
    #             "first_cycle_structures": cycle_pairing["first_cycle"]["structures"],
    #             "second_cycle_order": cycle_pairing["second_cycle"]["order"],
    #         }
    #     )
    return grouped_cycle_pairings


def main():
    all_cycle_pairings_result = all_cycle_pairings(3)
    filtered_cycle_pairings = filter_redundant_cycle_pairings(all_cycle_pairings_result)
    # grouped_cycle_pairings = group_cycle_pairings(filtered_cycle_pairings)
    return filtered_cycle_pairings


if __name__ == "__main__":
    with open("./output.txt", "w") as f:
        f.write(str(main()) + "\n")