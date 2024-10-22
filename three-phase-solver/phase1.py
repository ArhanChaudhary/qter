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


def signature(partition):
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
    for edges in range(2, EDGES + 1):
        for first_edge_count in range(1, edges):
            second_edge_count = edges - first_edge_count
            first_edge_partitions = integer_partitions(first_edge_count)
            second_edge_partitions = integer_partitions(second_edge_count)

            for corners in range(2, CORNERS + 1):
                for first_corner_count in range(1, corners):
                    second_corner_count = corners - first_corner_count
                    first_corner_partitions = integer_partitions(first_corner_count)
                    second_corner_partitions = integer_partitions(second_corner_count)

                    for first_cycle in highest_order_cycles_from_partitions(
                        first_edge_partitions, first_corner_partitions
                    ):
                        first_cycle["structures"] = all_cycle_structures(first_cycle)
                        if first_cycle["structures"] == set():
                            continue

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
                                if second_cycle["structures"] == set():
                                    continue

                                cycle_pairing = {
                                    "dim": (edges, corners),
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
    always_orient_edges = [
        i
        for i, permutation_order in enumerate(cycle["edge_partition"])
        if permutation_order == 1
    ]
    always_orient_corners = [
        i
        for i, permutation_order in enumerate(cycle["corner_partition"])
        if permutation_order == 1
    ]
    for edge_orientation_mask in orientation_masks(
        [ORIENT, NO_ORIENT], len(cycle["edge_partition"])
    ):
        if (
            any(edge_orientation_mask[i] == NO_ORIENT for i in always_orient_edges)
            # cannot flip an odd number of edges
            or edge_orientation_mask.count(ORIENT) % 2 == 1
            or (
                cycle["critical_orient_edge_indicies"] is not None
                # at least one cycle with the highest p-adic valuation has oriented
                # so the LCM doesn't lessen
                and all(
                    edge_orientation_mask[i] == NO_ORIENT
                    for i in cycle["critical_orient_edge_indicies"]
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
                    for i in always_orient_corners
                )
                # cannot flip exactly one corner
                or corner_orientation_mask.count(ORIENT) == 1
                or (
                    cycle["critical_orient_corner_indicies"] is not None
                    # read above
                    and all(
                        corner_orientation_mask[i] == NO_ORIENT
                        for i in cycle["critical_orient_corner_indicies"]
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
    return cycle_structures


def highest_order_cycles_from_partitions(edge_partitions, corner_partitions):
    highest_order = 1
    ret = []
    for edge_partition in edge_partitions:
        orient_edges = len(edge_partition) > 1
        for corner_partition in corner_partitions:
            if signature(corner_partition) != signature(edge_partition):
                continue
            orient_corners = len(corner_partition) > 1
            # k * lcm(a, b, c ...) = lcm(ka, kb, kc ...) (best case that is valid)
            order = math.lcm(
                conditional_edge_factor(orient_edges) * math.lcm(*edge_partition),
                conditional_corner_factor(orient_corners) * math.lcm(*corner_partition),
            )
            if order < highest_order:
                continue
            if order > highest_order:
                ret = []
            highest_order = order
            if orient_edges:
                max_p_adic_valuation = max(
                    p_adic_valuation(permutation_order, 2)
                    for permutation_order in edge_partition
                )
                critical_orient_edge_indicies = [
                    i
                    for i, permutation_order in enumerate(edge_partition)
                    if p_adic_valuation(permutation_order, 2) == max_p_adic_valuation
                ]
            else:
                critical_orient_edge_indicies = None

            if orient_corners:
                max_p_adic_valuation = max(
                    p_adic_valuation(permutation_order, 3)
                    for permutation_order in corner_partition
                )
                critical_orient_corner_indicies = [
                    i
                    for i, permutation_order in enumerate(corner_partition)
                    if p_adic_valuation(permutation_order, 3) == max_p_adic_valuation
                ]
            else:
                critical_orient_corner_indicies = None

            ret.append(
                {
                    "order": highest_order,
                    "edge_partition": edge_partition,
                    "corner_partition": corner_partition,
                    "critical_orient_edge_indicies": critical_orient_edge_indicies,
                    "critical_orient_corner_indicies": critical_orient_corner_indicies,
                }
            )
    return ret


def filter_redundant_cycle_pairings(cycle_pairings):
    filtered_cycle_pairings = []
    for maybe_redundant in sorted(
        cycle_pairings,
        key=operator.itemgetter("order_product"),
        reverse=True,
    ):
        if any(
            redundant_cycle_pairing(not_redundant, maybe_redundant)
            for not_redundant in filtered_cycle_pairings
        ):
            continue
        filtered_cycle_pairings.append(maybe_redundant)
    return filtered_cycle_pairings


def main():
    all_cycle_pairings_result = all_cycle_pairings()
    filtered_cycle_pairings = filter_redundant_cycle_pairings(all_cycle_pairings_result)
    with open("./output.txt", "w") as f:
        f.write(str(filtered_cycle_pairings) + "\n")


if __name__ == "__main__":
    main()
