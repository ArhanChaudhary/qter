import math
import operator

a = [
    [
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
        (28, (2, 2, 7)),
        (24, (1, 3, 4, 4)),
        (24, (2, 3, 3, 4)),
        (24, (1, 1, 1, 2, 3, 4)),
        (24, (1, 1, 4, 6)),
        (22, (1, 11)),
        (18, (1, 1, 1, 9)),
        (18, (3, 9)),
        (16, (4, 8)),
        (16, (1, 1, 2, 8)),
    ],
    [
        (45, (3, 5)),
        (36, (1, 3, 4)),
        (30, (1, 2, 5)),
        (24, (8,)),
        (21, (1, 7)),
        (18, (1, 2, 2, 3)),
        (18, (2, 6)),
        (12, (1, 1, 2, 4)),
        (12, (4, 4)),
    ],
]


def sign(x):
    return (sum(x) - len(x)) % 2


def iterative_process_desc(all_reduced_integer_partitions):
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

    stack = [(len(all_reduced_integer_partitions) - 1, 1, None)]
    while stack:
        i, running_order, partition = stack.pop()
        if partition is not None:
            cubie_partition_objs[i + 1] = partition
        if i == -1:
            if sum(map(sign, cubie_partition_objs)) % 2 != 0:
                continue
            if running_order > highest_order:
                cycles = []
            if running_order < highest_order:
                continue
            highest_order = running_order
            cycles.append(cubie_partition_objs.copy())
            continue
        for lcm_and_partition in all_reduced_integer_partitions[i]:
            lcm, partition = lcm_and_partition
            rest_upper_bound = running_order * lcm
            if rest_upper_bound * rest_upper_bounds[i] < highest_order:
                break
            stack.append(
                (i - 1, rest_upper_bound // math.gcd(running_order, lcm), partition)
            )

    return highest_order, cycles


print(iterative_process_desc(a))

