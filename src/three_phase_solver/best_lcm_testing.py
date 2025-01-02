from timeit import default_timer
import math
import heapq
import operator

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

_6x6 = [
    corners_constraint,
    s24_noconstraint,
    s24_noconstraint,
    s24_noconstraint,
    s24_noconstraint,
    s24_noconstraint,
    s24_noconstraint,
]


_5x5 = [
    edges_constraint,
    corners_constraint,
    s24_constraint,
    s24_constraint,
    s24_constraint,
]

_4x4 = [
    corners_constraint,
    s24_noconstraint,
    s24_constraint,
]

_3x3 = [
    edges_constraint,
    corners_constraint,
]


def sign(x):
    return (sum(x) - len(x)) % 2


def iterative_process_desc(all_reduced_integer_partitions, g, org=False):
    c = 0
    highest_order = -1
    rest_upper_bounds = []
    cycles = []
    cubie_partition_objs = [None] * len(all_reduced_integer_partitions)
    rest_upper_bound = 1

    for lcm_and_partition in map(
        operator.itemgetter(0), all_reduced_integer_partitions
    ):
        rest_upper_bounds.append(rest_upper_bound)
        rest_upper_bound *= lcm_and_partition[0]

    heap = []
    if org:
        heap.append((len(all_reduced_integer_partitions) - 1, 1, None))
    else:
        heapq.heappush(heap, (len(all_reduced_integer_partitions) - 1, 1, 1, None))
    while heap:
        if org:
            i, running_order, prev_partition = heap.pop()
        else:
            p = heapq.heappop(heap)
            i, _gcd, running_order, prev_partition = p
        if prev_partition is not None:
            cubie_partition_objs[i + 1] = prev_partition
        if i == -1:
            # if sum(map(sign, cubie_partition_objs)) % 2 != 0:
            #     continue
            if running_order > highest_order:
                cycles.clear()
            if running_order < highest_order:
                continue
            highest_order = running_order
            cycles.append(cubie_partition_objs.copy())
            continue
        for lcm_and_partition in all_reduced_integer_partitions[i]:
            c += 1
            lcm, partition = lcm_and_partition
            rest_upper_bound = running_order * lcm
            if rest_upper_bound * rest_upper_bounds[i] < highest_order:
                break
            gcd = math.gcd(running_order, lcm)
            if i > 1 and (
                gcd != 1 or prev_partition is not None and partition > prev_partition
            ):
                continue
            if org:
                heap.append((i - 1, rest_upper_bound // gcd, partition))
            else:
                heapq.heappush(
                    heap,
                    (
                        i - 1,
                        gcd,
                        rest_upper_bound // gcd,
                        partition,
                    ),
                )
    print("Worst: ", math.prod(map(len, all_reduced_integer_partitions)))
    print("Actual: ", c)
    return highest_order, cycles

print("NEW:\n")

start = default_timer()
print(iterative_process_desc(_5x5, 1, False))
print(default_timer() - start)
start = default_timer()
print(iterative_process_desc(_6x6, 0, False))
print(default_timer() - start)

print("\nORG:\n")

start2 = default_timer()
print(iterative_process_desc(_5x5, 1, True))
print(default_timer() - start2)
start2 = default_timer()
print(iterative_process_desc(_6x6, 0, True))
print(default_timer() - start2)
