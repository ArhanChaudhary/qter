import math


# Taken from https://stackoverflow.com/a/10036764/12230735
def partition(n):
    answer = {(n,)}
    for x in range(1, n):
        for y in partition(n - x):
            answer.add(tuple(sorted((x,) + y)))
    return answer


def sign(partition: tuple[int, ...]):
    return (-1) ** (sum(k - 1 for k in partition))


def order_products():
    product_data = []
    for first_cycle_edge_count in range(1, 11):
        for first_cycle_corner_count in range(1, 7):
            first_cycle = highest_order_cycle_inplace_components(
                first_cycle_edge_count, first_cycle_corner_count
            )
            for second_cycle_edge_count in range(
                1, min(first_cycle_edge_count, 11 - first_cycle_edge_count) + 1
            ):
                for second_cycle_corner_count in range(
                    1, min(first_cycle_corner_count, 7 - first_cycle_corner_count) + 1
                ):
                    second_cycle = highest_order_cycle_inplace_components(
                        second_cycle_edge_count,
                        second_cycle_corner_count,
                    )
                    product = second_cycle["order"] * first_cycle["order"]
                    product_data.append(
                        {
                            "product": product,
                            "first_cycle": first_cycle,
                            "second_cycle": second_cycle,
                        }
                    )
    product_data.sort(key=lambda p: p["product"], reverse=True)
    return product_data


def highest_order_cycle_inplace_components(edge_count, corner_count):
    edge_partitions = {i for i in partition(edge_count + 1) if 1 in i}
    corner_partitions = {i for i in partition(corner_count + 1) if 1 in i}
    highest_order = 1
    combined_cycle_structure = []
    for corner_partition in corner_partitions:
        for edge_partition in edge_partitions:
            valid = sign(corner_partition) == sign(edge_partition)
            if not valid:
                continue
            order = math.lcm(
                2 * math.lcm(*edge_partition), 3 * math.lcm(*corner_partition)
            )
            if order > highest_order:
                highest_order = order
                edge_cycle_orders = [i * 2 for i in edge_partition]
                corner_cycle_orders = [i * 3 for i in corner_partition]
                combined_cycle_structure = [0] * (
                    max(*edge_cycle_orders, *corner_cycle_orders) - 1
                )
                for order in edge_cycle_orders:
                    combined_cycle_structure[order - 2] += 1
                for order in corner_cycle_orders:
                    combined_cycle_structure[order - 2] += 1
    return {
        "order": highest_order,
        "structure": str(combined_cycle_structure).replace(" 0", ""),
    }


def main():
    with open("output.txt", "w") as f:
        # f.write(str(order_products()))
        for product in order_products():
            if product["product"] > 4000:
                f.write(str(product) + "\n")


if __name__ == "__main__":
    main()
