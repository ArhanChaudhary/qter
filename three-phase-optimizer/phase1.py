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


def redundant_order_pairing(not_redundant, maybe_redundant):
    a = not_redundant["first_cycle"]["order"]
    b = not_redundant["second_cycle"]["order"]
    c = maybe_redundant["first_cycle"]["order"]
    d = maybe_redundant["second_cycle"]["order"]
    return (
        (c <= a and d < b)
        or (c < a and d <= b)
        or (d <= a and c < b)
        or (d < a and c <= b)
    )


def all_cycle_pairings():
    cycle_pairings = []
    # 2 because integer_partitions(1) returns {(1,)} and any cubie of permutation
    # order 1 doesnt orient and therefore is not a cycle
    for first_edge_count in range(2, EDGES - 1):
        second_edge_count = EDGES - first_edge_count
        first_edge_partitions = integer_partitions(first_edge_count)
        second_edge_partitions = integer_partitions(second_edge_count)
        shared_second_edge_partitions = {(1,) + i for i in second_edge_partitions}

        for first_corner_count in range(2, CORNERS - 1):
            second_corner_count = CORNERS - first_corner_count
            first_corner_partitions = integer_partitions(first_corner_count)
            second_corner_partitions = integer_partitions(second_corner_count)
            shared_second_corner_partitions = {
                (1,) + i for i in second_corner_partitions
            }

            first_cycle = highest_order_cycle_from_partitions(
                first_edge_partitions, first_corner_partitions
            )
            first_cycle["structures"] = all_cycle_structures(first_cycle)

            share_edge = 1 in first_cycle["edge_partition"]
            share_corner = 1 in first_cycle["corner_partition"]
            second_cycle = highest_order_cycle_from_partitions(
                shared_second_edge_partitions if share_edge else second_edge_partitions,
                shared_second_corner_partitions
                if share_corner
                else second_corner_partitions,
            )
            second_cycle["structures"] = all_cycle_structures(second_cycle)
            cycle_pairings.append(
                {
                    "order_product": first_cycle["order"] * second_cycle["order"],
                    "first_cycle": first_cycle,
                    "second_cycle": second_cycle,
                }
            )
    return cycle_pairings


def all_cycle_structures(cycle):
    edge_partition = cycle["edge_partition"]
    corner_partition = cycle["corner_partition"]
    always_orient_edge_index = cycle["always_orient_edge_index"]
    always_orient_corner_index = cycle["always_orient_corner_index"]

    cycle_structures = set()
    for edge_orientation_mask in orientation_masks(
        [ORIENT, NO_ORIENT], len(edge_partition)
    ):
        if (
            # cannot flip an odd number of edges
            edge_orientation_mask.count(ORIENT) % 2 == 1
            # always orient the cycle with the highest p-adic valuation so the
            # LCM doesn't lessen
            or always_orient_edge_index is not None
            and edge_orientation_mask[always_orient_edge_index] == NO_ORIENT
        ):
            continue
        for corner_orientation_mask in orientation_masks(
            [ORIENT, NO_ORIENT], len(corner_partition)
        ):
            if (
                # cannot flip exactly one corner
                corner_orientation_mask.count(ORIENT) == 1
                # read above
                or always_orient_corner_index is not None
                and corner_orientation_mask[always_orient_corner_index] == NO_ORIENT
            ):
                continue
            edge_cycle_orders = [
                cycle_order
                * conditional_edge_factor(edge_orientation_mask[i] == ORIENT)
                for i, cycle_order in enumerate(edge_partition)
            ]
            corner_cycle_orders = [
                cycle_order
                * conditional_corner_factor(corner_orientation_mask[i] == ORIENT)
                for i, cycle_order in enumerate(corner_partition)
            ]

            cycle_structure = [0] * (max(*edge_cycle_orders, *corner_cycle_orders) - 1)
            for i, order in enumerate(edge_cycle_orders):
                if order >= 2:
                    cycle_structure[order - 2] += conditional_edge_factor(
                        edge_orientation_mask[i] == NO_ORIENT
                    )
            for i, order in enumerate(corner_cycle_orders):
                if order >= 2:
                    cycle_structure[order - 2] += conditional_corner_factor(
                        corner_orientation_mask[i] == NO_ORIENT
                    )
            cycle_structures.add(tuple(cycle_structure))

    return cycle_structures


def highest_order_cycle_from_partitions(edge_partitions, corner_partitions):
    highest_order = 1
    highest_order_edge_partition = ()
    highest_order_corner_partition = ()
    always_orient_edge_index = None
    always_orient_corner_index = None
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
            if order <= highest_order:
                continue
            highest_order = order
            highest_order_edge_partition = edge_partition
            highest_order_corner_partition = corner_partition
            if orient_edges:
                always_orient_edge_index, _ = max(
                    (
                        (i, p_adic_valuation(cycle_order, 2))
                        for i, cycle_order in enumerate(highest_order_edge_partition)
                    ),
                    key=operator.itemgetter(1),
                )
            if orient_corners:
                always_orient_corner_index, _ = max(
                    (
                        (i, p_adic_valuation(cycle_order, 3))
                        for i, cycle_order in enumerate(highest_order_corner_partition)
                    ),
                    key=operator.itemgetter(1),
                )
    return {
        "order": highest_order,
        "edge_partition": highest_order_edge_partition,
        "corner_partition": highest_order_corner_partition,
        "always_orient_edge_index": always_orient_edge_index,
        "always_orient_corner_index": always_orient_corner_index,
    }


def filter_redundant_cycle_pairings(cycle_pairings):
    filtered_cycle_pairings = []
    for maybe_redundant in sorted(
        cycle_pairings,
        key=operator.itemgetter("order_product"),
        reverse=True,
    ):
        if any(
            redundant_order_pairing(not_redundant, maybe_redundant)
            for not_redundant in filtered_cycle_pairings
        ):
            continue
        filtered_cycle_pairings.append(maybe_redundant)
    return filtered_cycle_pairings


def group_cycle_pairings(cycle_pairings):
    grouped_cycle_pairings = []
    for cycle_pairing in cycle_pairings:
        for grouped_cycle_pairing in (
            {
                "first_cycle_order": cycle_pairing["first_cycle"]["order"],
                "first_cycle_structure": cycle_pairing["first_cycle"]["structures"],
                "second_cycle_order": cycle_pairing["second_cycle"]["order"],
            },
            {
                "first_cycle_order": cycle_pairing["second_cycle"]["order"],
                "first_cycle_structure": cycle_pairing["second_cycle"]["structures"],
                "second_cycle_order": cycle_pairing["first_cycle"]["order"],
            },
        ):
            if existing := next(
                (
                    grouped_cycle_pairing_iter
                    for grouped_cycle_pairing_iter in grouped_cycle_pairings
                    if grouped_cycle_pairing_iter["first_cycle_order"]
                    == grouped_cycle_pairing["first_cycle_order"]
                    and grouped_cycle_pairing_iter["second_cycle_order"]
                    == grouped_cycle_pairing["second_cycle_order"]
                ),
                None,
            ):
                existing["first_cycle_structure"].update(
                    grouped_cycle_pairing["first_cycle_structure"]
                )
            else:
                grouped_cycle_pairings.append(grouped_cycle_pairing)
    return grouped_cycle_pairings


def main():
    all_cycle_pairings_result = all_cycle_pairings()
    filtered_cycle_pairings = filter_redundant_cycle_pairings(all_cycle_pairings_result)
    grouped_cycle_pairings = group_cycle_pairings(filtered_cycle_pairings)
    with open("output.txt", "w") as f:
        f.write(str(grouped_cycle_pairings) + "\n")


if __name__ == "__main__":
    main()
