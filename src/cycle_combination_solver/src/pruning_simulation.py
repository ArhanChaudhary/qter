import random
import math


CORNERS_DIST = [
    1,
    18,
    243,
    2874,
    28000,
    205416,
    1168516,
    5402628,
    20776176,
    45391616,
    15139616,
    64736,
]

dists = [
    1,
    18,
    243,
    3240,
    43239,
    574908,
    7618438,
    100803036,
    1332343288,
    17596479795,
    232248063316,
    3063288809012,
    40374425656248,
    531653418284628,
    6989320578825358,
    91365146187124320,
    1100000000000000000,
    12000000000000000000,
    29000000000000000000,
    1500000000000000000,
    490000000,
]


def uniform_random(frac):
    return random.uniform(0, 1) < frac


def random_corner(half=False):
    random_number = random.randint(1, sum(CORNERS_DIST))
    if half and uniform_random(1 - EXACT_FILLED):
        return 0
    cumulative_sum = 0
    for i, num in enumerate(CORNERS_DIST):
        cumulative_sum += num
        if random_number <= cumulative_sum:
            return i
    return len(CORNERS_DIST) - 1


def actual_half_corner():
    all_random_numbers = [random_corner(True) for _ in range(100000)]
    return sum(all_random_numbers) / len(all_random_numbers)


def actual_approx_corner():
    all_random_numbers = []
    for _ in range(100000):
        frac, int_ = math.modf(1 / EXACT_FILLED)
        assert int_ != 0
        random_number = min(random_corner() for _ in range(int(int_)))
        if uniform_random(frac):
            random_number = min(random_number, random_corner())

        all_random_numbers.append(random_number)
    return sum(all_random_numbers) / len(all_random_numbers)


EXACT_FILLED = 0


def main():
    global EXACT_FILLED
    for _ in range(19):
        EXACT_FILLED += 0.05
        print(
            f"{EXACT_FILLED:.2f}: {actual_half_corner():.2f} {actual_approx_corner():.2f}"
        )

    BRANCHING_FACTOR = 6 + math.sqrt(6) * 3
    SIZE = 43252003274489860000

    pos_at_depth = 1
    pos_seen = 1
    depth = 0

    while pos_seen < SIZE:
        print(f"{int(pos_at_depth)}\n{int(dists[depth])}\n")
        if pos_at_depth == 1:
            pos_at_depth = 18
        else:
            pos_at_depth *= BRANCHING_FACTOR * math.exp(-5.4768 * pos_seen / SIZE)
        pos_seen += pos_at_depth
        depth += 1


if __name__ == "__main__":
    main()
