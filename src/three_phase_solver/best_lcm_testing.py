from timeit import default_timer
import heapq
import math
import operator

# outputs of reduced_integer_partitions on 8, 12, and 24
# a suffix with `constraint` indicates that partitions must
# have equal signatures for the lesser LCM to be filtered out

edges_constraint = [
    (120, (3, 4, 5)),
    (84, (2, 3, 7)),
    (70, (5, 7)),
    (60, (2, 2, 3, 5)),
    (56, (1, 4, 7)),
    (48, (1, 3, 8)),
    (42, (1, 1, 3, 7)),
    (40, (1, 2, 4, 5)),
    (36, (1, 2, 9)),
    (28, (1, 2, 2, 7)),
    (24, (1, 3, 4, 4)),
    (24, (2, 3, 3, 4)),
    (24, (1, 1, 1, 2, 3, 4)),
    (24, (1, 1, 4, 6)),
    (22, (1, 11)),
    (18, (1, 1, 1, 9)),
    (18, (3, 9)),
    (16, (4, 8)),
    (16, (1, 1, 2, 8)),
]

corners_constraint = [
    (45, (3, 5)),
    (36, (1, 3, 4)),
    (30, (1, 2, 5)),
    (21, (1, 7)),
    (18, (1, 2, 2, 3)),
    (18, (2, 6)),
    (12, (1, 1, 2, 4)),
    (12, (4, 4)),
    (8, (8,)),
]

s24_noconstraint = [
    (840, (1, 3, 5, 7, 8)),
    (660, (1, 3, 4, 5, 11)),
    (630, (1, 2, 5, 7, 9)),
    (504, (7, 8, 9)),
    (462, (6, 7, 11)),
    (462, (1, 2, 3, 7, 11)),
    (440, (5, 8, 11)),
    (396, (4, 9, 11)),
    (390, (5, 6, 13)),
    (390, (1, 2, 3, 5, 13)),
    (385, (1, 5, 7, 11)),
    (364, (4, 7, 13)),
    (360, (2, 5, 8, 9)),
    (360, (1, 1, 5, 8, 9)),
    (312, (3, 8, 13)),
    (308, (1, 1, 4, 7, 11)),
    (308, (2, 4, 7, 11)),
    (273, (1, 3, 7, 13)),
    (264, (1, 1, 3, 8, 11)),
    (264, (2, 3, 8, 11)),
    (260, (2, 4, 5, 13)),
    (260, (1, 1, 4, 5, 13)),
    (240, (3, 5, 16)),
    (234, (2, 9, 13)),
    (204, (3, 4, 17)),
    (170, (2, 5, 17)),
    (143, (11, 13)),
    (119, (7, 17)),
    (114, (2, 3, 19)),
    (112, (1, 7, 16)),
    (95, (5, 19)),
    (76, (1, 4, 19)),
    (23, (1, 23)),
]

s24_constraint = [
    (840, (1, 3, 5, 7, 8)),
    (660, (1, 3, 4, 5, 11)),
    (630, (1, 2, 5, 7, 9)),
    (504, (7, 8, 9)),
    (462, (6, 7, 11)),
    (462, (1, 2, 3, 7, 11)),
    (440, (5, 8, 11)),
    (420, (1, 1, 4, 5, 6, 7)),
    (420, (1, 1, 1, 2, 3, 4, 5, 7)),
    (420, (3, 4, 7, 10)),
    (420, (1, 3, 4, 4, 5, 7)),
    (420, (2, 3, 3, 4, 5, 7)),
    (396, (4, 9, 11)),
    (390, (5, 6, 13)),
    (390, (1, 2, 3, 5, 13)),
    (385, (1, 5, 7, 11)),
    (364, (4, 7, 13)),
    (360, (2, 5, 8, 9)),
    (360, (1, 1, 5, 8, 9)),
    (330, (1, 2, 2, 3, 5, 11)),
    (330, (2, 5, 6, 11)),
    (315, (1, 1, 1, 5, 7, 9)),
    (315, (3, 5, 7, 9)),
    (312, (3, 8, 13)),
    (308, (1, 1, 4, 7, 11)),
    (308, (2, 4, 7, 11)),
    (280, (4, 5, 7, 8)),
    (280, (1, 1, 2, 5, 7, 8)),
    (273, (1, 3, 7, 13)),
    (264, (1, 1, 3, 8, 11)),
    (264, (2, 3, 8, 11)),
    (260, (2, 4, 5, 13)),
    (260, (1, 1, 4, 5, 13)),
    (252, (4, 4, 7, 9)),
    (252, (1, 1, 2, 4, 7, 9)),
    (240, (3, 5, 16)),
    (234, (2, 9, 13)),
    (231, (3, 3, 7, 11)),
    (231, (1, 1, 1, 3, 7, 11)),
    (220, (4, 4, 5, 11)),
    (220, (1, 1, 2, 4, 5, 11)),
    (204, (3, 4, 17)),
    (198, (2, 2, 9, 11)),
    (195, (1, 1, 1, 3, 5, 13)),
    (195, (3, 3, 5, 13)),
    (182, (2, 2, 7, 13)),
    (170, (2, 5, 17)),
    (168, (1, 1, 1, 6, 7, 8)),
    (168, (3, 6, 7, 8)),
    (168, (1, 1, 3, 4, 7, 8)),
    (168, (2, 2, 2, 3, 7, 8)),
    (168, (1, 2, 3, 3, 7, 8)),
    (168, (1, 1, 1, 1, 2, 3, 7, 8)),
    (156, (3, 4, 4, 13)),
    (156, (1, 1, 2, 3, 4, 13)),
    (156, (1, 4, 6, 13)),
    (143, (11, 13)),
    (119, (7, 17)),
    (117, (1, 1, 9, 13)),
    (114, (2, 3, 19)),
    (112, (1, 7, 16)),
    (104, (1, 2, 8, 13)),
    (102, (2, 2, 3, 17)),
    (95, (5, 19)),
    (85, (1, 1, 5, 17)),
    (80, (1, 2, 5, 16)),
    (76, (1, 4, 19)),
    (68, (1, 2, 4, 17)),
    (57, (1, 1, 3, 19)),
    (48, (1, 1, 1, 2, 3, 16)),
    (48, (2, 3, 3, 16)),
    (48, (1, 3, 4, 16)),
    (48, (1, 1, 6, 16)),
    (38, (1, 2, 2, 19)),
    (23, (1, 23)),
]

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
def highest_order_partitions(puzzle, big_cube):
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
    while heap:
        _, i, running_order, cubie_partition_objs = heapq.heappop(heap)

        if i == -1:
            if running_order > highest_order:
                cycles.clear()
            if running_order < highest_order:
                continue
            highest_order = running_order
            cycles.append(cubie_partition_objs)
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
# TEST CASE: _6x6 should generate 7742 unique results
results = highest_order_partitions(_6x6, False)
end = default_timer() - start
print(f"Generated {len(results[1])} unique results in {end:.3g}s\n")
if WRITE_TO_FILE:
    with open("output.py", "w") as f:
        f.write(
            f"# Highest order: {results[0]}\n# Run `python -i output.py`\nresults = {results[1]}"
        )
else:
    print(results)
