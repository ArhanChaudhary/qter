import itertools
import math
import operator
import functools

EDGES = 12
CORNERS = 8
NO_ORIENT = 0
ORIENT = 1


# Taken from https://stackoverflow.com/a/10036764/12230735
@functools.lru_cache
def integer_partitions(n):
    answer = {(n,)}
    for x in range(1, n):
        for y in integer_partitions(n - x):
            answer.add(tuple(sorted((x,) + y)))
    return answer


def share_partitions(partitions):
    return {(1,) + i for i in partitions}


def p_adic_valuation(n, p):
    exponent = 0
    while n % p == 0 and n != 0:
        n //= p
        exponent += 1
    return exponent


def sign(partition):
    return (-1) ** sum(k - 1 for k in partition)


def conditional_edge_factor(cond):
    return 2 if cond else 1


def conditional_corner_factor(cond):
    return 3 if cond else 1


def orientation_masks(masks, mask_length):
    return [] if mask_length == 0 else itertools.product(masks, repeat=mask_length)


def redundant_cycle_pairing(not_redundant, maybe_redundant):
    nfo = not_redundant["first_cycle"]["order"]
    nso = not_redundant["second_cycle"]["order"]
    mfo = maybe_redundant["first_cycle"]["order"]
    mso = maybe_redundant["second_cycle"]["order"]
    redundant_order_pairing = (
        (mfo <= nfo and mso < nso)
        or (mfo < nfo and mso <= nso)
        or (mso <= nfo and mfo < nso)
        or (mso < nfo and mfo <= nso)
    )
    if redundant_order_pairing:
        return True

    mfe = maybe_redundant["first_cycle"]["edge_partition"]
    mfc = maybe_redundant["first_cycle"]["corner_partition"]
    mse = maybe_redundant["second_cycle"]["edge_partition"]
    msc = maybe_redundant["second_cycle"]["corner_partition"]
    nfe = not_redundant["first_cycle"]["edge_partition"]
    nfc = not_redundant["first_cycle"]["corner_partition"]
    nse = not_redundant["second_cycle"]["edge_partition"]
    nsc = not_redundant["second_cycle"]["corner_partition"]
    redundant_partition_pairing = (
        maybe_redundant["share_edge"] == not_redundant["share_edge"]
        and maybe_redundant["share_corner"] == not_redundant["share_corner"]
        and (
            mfe == nfe
            and mfc == nfc
            and mse == nse
            and msc == nsc
            or mfe == nse
            and mfc == nsc
            and mse == nfe
            and msc == nfc
        )
    )
    return redundant_partition_pairing


def all_cycle_pairings():
    cycle_pairings = []
    for used_edge_count in range(2, EDGES + 1):
        for first_edge_count in range(1, used_edge_count):
            second_edge_count = used_edge_count - first_edge_count
            first_edge_partitions = integer_partitions(first_edge_count)
            second_edge_partitions = integer_partitions(second_edge_count)

            for used_corner_count in range(2, CORNERS + 1):
                for first_corner_count in range(1, used_corner_count):
                    second_corner_count = used_corner_count - first_corner_count
                    first_corner_partitions = integer_partitions(first_corner_count)
                    second_corner_partitions = integer_partitions(second_corner_count)

                    for first_cycle in highest_order_cycles_from_partitions(
                        first_edge_partitions, first_corner_partitions
                    ):
                        first_cycle["structures"] = all_cycle_structures(first_cycle)

                        share_mat = []
                        if (
                            second_edge_count >= first_edge_count
                            and second_corner_count >= first_corner_count
                        ):
                            share_mat.append((False, False))
                        if share_edge := 1 in first_cycle["edge_partition"]:
                            share_mat.append((True, False))
                        if share_corner := 1 in first_cycle["corner_partition"]:
                            share_mat.append((False, True))
                        if share_edge and share_corner:
                            share_mat.append((True, True))
                        for (
                            share_edge,
                            share_corner,
                        ) in share_mat:
                            for second_cycle in highest_order_cycles_from_partitions(
                                share_partitions(second_edge_partitions)
                                if share_edge
                                else second_edge_partitions,
                                share_partitions(second_corner_partitions)
                                if share_corner
                                else second_corner_partitions,
                            ):
                                second_cycle["structures"] = all_cycle_structures(
                                    second_cycle
                                )

                                cycle_pairing = {
                                    "used_edge_count": used_edge_count,
                                    "used_corner_count": used_corner_count,
                                    "share_edge": share_edge,
                                    "share_corner": share_corner,
                                    "order_product": first_cycle["order"]
                                    * second_cycle["order"],
                                }
                                if first_cycle["order"] < second_cycle["order"]:
                                    cycle_pairing["first_cycle"] = second_cycle
                                    cycle_pairing["second_cycle"] = first_cycle
                                else:
                                    cycle_pairing["first_cycle"] = first_cycle
                                    cycle_pairing["second_cycle"] = second_cycle
                                cycle_pairings.append(cycle_pairing)
    return cycle_pairings


def all_cycle_structures(cycle):
    cycle_structures = set()
    for edge_orientation_mask in orientation_masks(
        [ORIENT, NO_ORIENT], len(cycle["edge_partition"])
    ):
        if (
            any(
                edge_orientation_mask[i] == NO_ORIENT
                for i in cycle["always_orient_edges"]
            )
            # cannot flip an odd number of edges
            or edge_orientation_mask.count(ORIENT) % 2 == 1
            or not (
                cycle["critical_orient_edges"] is None
                # at least one cycle with the highest p-adic valuation has oriented
                # so the LCM doesn't lessen
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
                any(
                    corner_orientation_mask[i] == NO_ORIENT
                    for i in cycle["always_orient_corners"]
                )
                # cannot flip exactly one corner
                or corner_orientation_mask.count(ORIENT) == 1
                or not (
                    cycle["critical_orient_corners"] is None
                    # read above
                    or any(
                        corner_orientation_mask[i] == ORIENT
                        for i in cycle["critical_orient_corners"]
                    )
                )
            ):
                continue
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
                assert cycle_order != 1
                cycle_structure[cycle_order - 2] += conditional_edge_factor(
                    edge_orientation_mask[i] == NO_ORIENT
                )

            for i, cycle_order in enumerate(corner_cycle_orders):
                assert cycle_order != 1
                cycle_structure[cycle_order - 2] += conditional_corner_factor(
                    corner_orientation_mask[i] == NO_ORIENT
                )
            cycle_structures.add(tuple(cycle_structure))
    assert cycle_structures != set(), cycle
    return cycle_structures


def highest_order_cycles_from_partitions(edge_partitions, corner_partitions):
    highest_order = 1
    cycles = []
    for edge_partition in edge_partitions:
        for corner_partition in corner_partitions:
            if sign(corner_partition) != sign(edge_partition):
                continue

            always_orient_edges = []
            max_two_adic_valuation = -1
            for i, permutation_order in enumerate(edge_partition):
                curr_two_adic_valuation = p_adic_valuation(permutation_order, 2)
                if curr_two_adic_valuation > max_two_adic_valuation:
                    max_two_adic_valuation = curr_two_adic_valuation
                    critical_orient_edges = [i]
                elif curr_two_adic_valuation == max_two_adic_valuation:
                    critical_orient_edges.append(i)
                if permutation_order == 1:
                    always_orient_edges.append(i)
            orient_edge_count = len(always_orient_edges)
            critical_is_disjoint = critical_orient_edges is not None and all(
                i not in always_orient_edges for i in critical_orient_edges
            )
            if critical_is_disjoint:
                orient_edge_count += 1
            invalid_orient_edge_count = (
                orient_edge_count == len(edge_partition) and orient_edge_count % 2 == 1
            )
            if invalid_orient_edge_count:
                if not critical_is_disjoint:
                    continue
                assert len(critical_orient_edges) == 1, critical_orient_edges
                orient_edge_count -= 1
                critical_orient_edges = None

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

            order = math.lcm(
                conditional_edge_factor(not invalid_orient_edge_count)
                * math.lcm(*edge_partition),
                conditional_corner_factor(not invalid_orient_corner_count)
                * math.lcm(*corner_partition),
            )
            if order < highest_order:
                continue
            if order > highest_order:
                cycles = []
            highest_order = order
            cycles.append(
                {
                    "order": highest_order,
                    "edge_partition": edge_partition,
                    "corner_partition": corner_partition,
                    "critical_orient_edges": critical_orient_edges,
                    "critical_orient_corners": critical_orient_corners,
                    "always_orient_edges": always_orient_edges,
                    "always_orient_corners": always_orient_corners,
                }
            )
    return cycles


def filter_redundant_cycle_pairings(cycle_pairings):
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


def main():
    all_cycle_pairings_result = all_cycle_pairings()
    filtered_cycle_pairings = filter_redundant_cycle_pairings(all_cycle_pairings_result)
    return filtered_cycle_pairings


if __name__ == "__main__":
    main()
    with open("./output.txt", "w") as f:
        f.write(str(main()) + "\n")
