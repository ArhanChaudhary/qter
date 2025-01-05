import functools
from timeit import default_timer
import heapq
import math
import operator


def p_adic_valuation(n, p):
    exponent = 0
    while n % p == 0 and n != 0:
        n //= p
        exponent += 1
    return exponent


@functools.cache
def integer_partitions(n):
    if n == 0:
        return {()}
    answer = {(n,)}
    for x in range(1, n):
        for y in integer_partitions(n - x):
            answer.add(tuple(sorted((x,) + y)))
    return answer


def partition_order(partition, orientation_count):
    lcm = math.lcm(*partition)
    if orientation_count == 1:
        return lcm
    order = lcm

    always_orient = None
    critical_orient = None
    max_p_adic_valuation = -1

    for j, permutation_order in enumerate(partition):
        curr_p_adic_valuation = p_adic_valuation(
            permutation_order,
            orientation_count,
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

    orient_count = 0 if always_orient is None else len(always_orient)
    critical_is_disjoint = critical_orient is not None and (
        always_orient is None or all(j not in always_orient for j in critical_orient)
    )
    if critical_is_disjoint:
        orient_count += 1
    unorient_critical = orient_count == len(partition) and (
        orientation_count == 2
        and orient_count % 2 == 1
        or orientation_count > 2
        and orient_count == 1
    )
    if unorient_critical:
        if critical_is_disjoint:
            return order
        else:
            return None
    else:
        if orient_count == 0:
            return order
        else:
            return order * orientation_count


def full_integer_partitions(cycle_cubie_count, orientation_count):
    partitions = [
        (order, partition)
        for partition in integer_partitions(cycle_cubie_count)
        if (order := partition_order(partition, orientation_count)) is not None
    ]
    partitions.sort(reverse=True, key=operator.itemgetter(0))
    return partitions


def reduced_integer_partitions(cycle_cubie_count, orientation_count, parity_aware):
    partitions = full_integer_partitions(cycle_cubie_count, orientation_count)

    dominated = [False] * len(partitions)
    reduced_partitions = []
    for i in range(len(partitions)):
        if dominated[i]:
            continue
        partition = partitions[i]
        reduced_partitions.append(partition)
        for j in range(i + 1, len(partitions)):
            if (
                partition[0] % partitions[j][0] == 0
                and partition[0] != partitions[j][0]
                and (
                    not parity_aware
                    or (
                        sum(partition[1])
                        + len(partition[1])
                        + sum(partitions[j][1])
                        + len(partitions[j][1])
                    )
                    % 2
                    == 0
                )
            ):
                dominated[j] = True
    return reduced_partitions


# list of (order, partition of N)
# example:
# corners_constraint == [(45, (3, 5)), (36, (1, 3, 4)), (30, (1, 2, 5)), (21, (1, 7)), (18, (1, 2, 2, 3)), (18, (2, 6)), (12, (1, 1, 2, 4)), (12, (4, 4)), (8, (8,))]

edges_constraint = reduced_integer_partitions(12, 2, True)
corners_constraint = reduced_integer_partitions(8, 3, True)
s24_noconstraint = reduced_integer_partitions(24, 1, False)
s24_constraint = reduced_integer_partitions(24, 1, True)
# TODO: asher and I discussed needing the full integer partitions for larger
# cubes. this is unfortunately very very slow
# edges_constraint = full_integer_partitions(12, 2)
# corners_constraint = full_integer_partitions(8, 3)
# s24_noconstraint = full_integer_partitions(24, 1)
# s24_constraint = full_integer_partitions(24, 1)

# the first element of the tuple is the index where everything then and after
# wards are identical orbits (s24). This is used to enforce a constraint that
# the partitions must be in ascending order for these orbits to avoid duplicates


_3x3 = (
    2,
    [
        edges_constraint,
        corners_constraint,
    ],
)

_4x4 = (
    1,
    [
        corners_constraint,
        s24_noconstraint,
        s24_constraint,
    ],
)

_5x5 = (
    2,
    [
        edges_constraint,
        corners_constraint,
        s24_constraint,
        s24_constraint,
        s24_constraint,
    ],
)


_6x6 = (
    1,
    [
        corners_constraint,
        # these *should* be s24_constraint but it's really slow :(
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
    ],
)

# GOAL: make 7x7 and onwards fast

_7x7 = (
    2,
    [
        edges_constraint,
        corners_constraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
        s24_noconstraint,
    ],
)


# big_cube is unused
def highest_order_partitions(puzzle, debug, big_cube):
    identical_index, all_reduced_integer_partitions = puzzle
    count = 0
    highest_order = -1
    rest_upper_bounds = []
    cycles = []
    rest_upper_bound = 1

    for lcm_and_partition in map(
        operator.itemgetter(0), all_reduced_integer_partitions
    ):
        rest_upper_bounds.append(rest_upper_bound)
        rest_upper_bound *= lcm_and_partition[0]

    heap = []
    # NOTE: heapq is not efficient! there are more efficient priority queue
    # data structures that exist (strict fibonacci heaps) but we use heapq
    # for simplicity.
    heapq.heappush(heap, (1, len(all_reduced_integer_partitions) - 1, 1, []))
    if debug:
        t = 0
    while heap:
        if debug:
            if t % 10000 == 0:
                print(f"The heap has {len(heap)} elements")
            t += 1
        _, i, running_order, cubie_partition_objs = heapq.heappop(heap)

        if i == -1:
            if running_order > highest_order:
                cycles.clear()
            if running_order < highest_order:
                continue
            highest_order = running_order
            cycles.append(cubie_partition_objs)
            if debug:
                print(f"New highest order: {highest_order}")
                print(f"Cycles: {cycles}")
            continue

        for lcm_and_partition in all_reduced_integer_partitions[i]:
            count += 1
            lcm, partition = lcm_and_partition
            rest_upper_bound = running_order * lcm
            if rest_upper_bound * rest_upper_bounds[i] < highest_order:
                break
            gcd = math.gcd(running_order, lcm)
            if (
                # does the current index refer to an identical orbit (s24)
                i >= identical_index
                # if so, then enforce p1 < p2 < p3 ... < pn for all partitions
                # to ensure no duplicates are generated. It is assumed that the
                # caller will manually permute these identical partitions/
                # TODO: how should duplicates be handled?
                and cubie_partition_objs
                and partition > cubie_partition_objs[0]
            ):
                continue
            heapq.heappush(
                heap,
                (
                    # adding `gcd` in front makes it faster, but not sure why
                    gcd,
                    # the order of the next two arguments is insignificant. I am
                    # getting slight performance improvements with this order.
                    i - 1,
                    rest_upper_bound // gcd,
                    # there are probably more efficient ways to create a new list
                    # every iteration, but I will leave it like this for the sake
                    # of not making the code more complicated than it already is
                    [partition] + cubie_partition_objs,
                ),
            )
    print(f"Took {count} loop iterations")
    return highest_order, cycles


WRITE_TO_FILE = True

start = default_timer()
results = highest_order_partitions(_5x5, True, False)
end = default_timer() - start
print(f"Generated {len(results[1])} unique results in {end:.3g}s")
if WRITE_TO_FILE:
    with open("output.py", "w") as f:
        f.write(
            f"# Highest order: {results[0]}\n# Run `python -i output.py`\n\nresults = {results[1]}"
        )
else:
    print(f"\nHighest order: {results[0]}\n{results[1]}")
