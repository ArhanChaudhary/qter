struct PrimePower {
    value: usize,
    pieces: usize,
}
struct OrderIteration {
    index: isize,
    piece_count: usize,
    product: usize,
    powers: Vec<usize>,
    pieces: Vec<usize>,
}

struct ComboIteration {
    register: usize,
    orbit: usize,
    orbit_sums: Vec<usize>,
    assignments: Vec<Assignment>,
    available_pieces: usize,
}

type Assignment = Vec<Vec<usize>>;

#[derive(Clone)]
struct PrimeCombo {
    order: usize,
    values: Vec<usize>,
    //piece_total: usize,
    piece_counts: Vec<usize>,
}

struct Orbit {
    name: String,
    cubie_count: usize,
    orient_count: usize,
    orient_sum: usize,
}

struct Partition {
    name: String,
    partition: Vec<usize>,
    order: usize,
}

struct Cycle {
    order: usize,
    partitions: Vec<Partition>,
}

struct CycleCombination {
    used_cubie_counts: Vec<usize>,
    order_product: usize,
    cycles: Vec<Cycle>,
}

fn prime_powers_below_n(n: usize, max_orient: &[usize]) -> Vec<Vec<PrimePower>> {
    let mut primes: Vec<usize> = vec![2];

    for possible_prime in (3..n + 1).step_by(2) {
        let mut is_prime = true;
        for p in primes.iter() {
            if p.pow(2) > possible_prime {
                break;
            }

            if possible_prime % p == 0 {
                is_prime = false;
                break;
            }
        }

        if !is_prime {
            continue;
        }

        primes.push(possible_prime);
    }

    let mut prime_powers = vec![];

    for (p, prime) in primes.iter().enumerate() {
        let mut orient: usize = 1;
        let mut piece_check = prime.pow(2);
        if max_orient.len() > *prime && max_orient[*prime] > 0 {
            orient = *prime;
            piece_check = *prime;
            prime_powers.push(vec![
                PrimePower {
                    value: 1,
                    pieces: 0,
                },
                PrimePower {
                    value: *prime,
                    pieces: 0,
                },
            ])
        } else {
            prime_powers.push(vec![
                PrimePower {
                    value: 1,
                    pieces: 0,
                },
                PrimePower {
                    value: *prime,
                    pieces: *prime,
                },
            ])
        }

        while piece_check <= n {
            prime_powers[p].push(PrimePower {
                value: orient * piece_check,
                pieces: piece_check,
            });

            piece_check *= *prime;
            if orient > 1 && piece_check > max_orient[*prime] {
                piece_check *= orient;
                orient = 1;
            }
        }
    }

    prime_powers
}

fn possible_order_list(
    total_pieces: usize,
    partition_max: usize,
    max_orient: &[usize],
) -> Vec<PrimeCombo> {
    let prime_powers = prime_powers_below_n(partition_max, max_orient);

    let mut paths = vec![];
    let mut stack: Vec<OrderIteration> = vec![OrderIteration {
        index: (prime_powers.len() - 1) as isize,
        piece_count: 0,
        product: 1,
        powers: vec![],
        pieces: vec![],
    }];

    while let Some(s) = stack.pop() {
        if s.index == -1 || prime_powers[s.index as usize][1].pieces + s.piece_count > total_pieces
        {
            paths.push(PrimeCombo {
                order: s.product,
                values: s.powers.clone(),
                //piece_total: s.pieces.clone().into_iter().sum(),
                piece_counts: s.pieces.clone(),
            });
            continue;
        }

        for p in prime_powers[s.index as usize].iter() {
            let new_pieces = s.piece_count
                + p.pieces
                + if p.pieces > 0 && p.pieces % 2 == 0 {
                    2
                } else {
                    0
                }; // TODO this should not happen on 4x4

            if new_pieces <= total_pieces {
                stack.push(OrderIteration {
                    index: s.index - 1,
                    piece_count: new_pieces,
                    product: s.product,
                    powers: s.powers.clone(),
                    pieces: s.pieces.clone(),
                });

                if p.value > 1 {
                    let s_last = stack.len() - 1;
                    stack[s_last].product *= p.value;
                    stack[s_last].powers.push(p.value);
                    stack[s_last].pieces.push(p.pieces);
                }
            }
        }
    }

    paths.sort_by(|a: &PrimeCombo, b: &PrimeCombo| b.order.partial_cmp(&a.order).unwrap());

    paths
}

fn cycle_combo_test(
    registers: &[PrimeCombo],
    cycle_cubie_counts: &[usize],
    puzzle_orbit_definition: &[Orbit],
) -> Option<Vec<Assignment>> {
    let mut stack: Vec<ComboIteration> = vec![ComboIteration {
        register: 0,
        orbit: 0,
        orbit_sums: vec![0; cycle_cubie_counts.len()],
        assignments: vec![vec![vec![]; cycle_cubie_counts.len()]; registers.len()],
        available_pieces: cycle_cubie_counts.iter().sum(), //TODO this is wrong
    }];

    let mut loops: usize = 0;
    while let Some(mut s) = stack.pop() {
        loops += 1;
        if loops > 1000 {
            return None;
        }

        let mut seen = vec![];
        while s.orbit == registers[s.register].values.len() {
            s.orbit = 0;
            s.register += 1;
            if s.register == registers.len() {
                break;
            }
        }

        if s.register == registers.len() {
            return Some(s.assignments);
        } //TODO no duplicates

        for (o, orbit) in puzzle_orbit_definition.iter().enumerate() {
            if orbit.orient_count == 1 {
                if seen.contains(&cycle_cubie_counts[o]) {
                    continue;
                } else {
                    seen.push(cycle_cubie_counts[o]);
                }
            }

            let mut new_cycle: usize;
            let new_available: usize;
            if orbit.orient_count > 1
                && registers[s.register].values[s.orbit] % orbit.orient_count == 0
            {
                new_cycle = registers[s.register].piece_counts[s.orbit];
                new_available = s.available_pieces;
            } else if registers[s.register].values[s.orbit]
                - registers[s.register].piece_counts[s.orbit]
                <= s.available_pieces
            {
                new_cycle = registers[s.register].values[s.orbit];
                new_available = s.available_pieces - registers[s.register].values[s.orbit]
                    + registers[s.register].piece_counts[s.orbit];
            } else {
                continue;
            }

            if new_cycle == 0 && s.assignments[s.register][o].is_empty() {
                if s.available_pieces == 0 {
                    continue;
                }
                new_cycle = 1;
            }

            let parity: usize = if new_cycle % 2 == 0 && new_cycle > 0 {
                2
            } else {
                0
            };

            if new_cycle + parity + s.orbit_sums[o] <= cycle_cubie_counts[o] {
                stack.push(ComboIteration {
                    register: s.register,
                    orbit: s.orbit + 1,
                    orbit_sums: s.orbit_sums.clone(),
                    assignments: s.assignments.clone(),
                    available_pieces: new_available,
                });

                if new_cycle > 0 {
                    let last = stack.len() - 1;
                    stack[last].orbit_sums[o] += new_cycle;
                    stack[last].assignments[s.register][o].push(new_cycle);
                    if parity > 0 {
                        stack[last].orbit_sums[o] += 2;
                        stack[last].assignments[s.register][o].push(2);
                    }
                }
            }
        }
    }

    None
}

fn assignments_to_combo(
    assignments: &mut [Vec<Vec<usize>>],
    registers: &[PrimeCombo],
    cycle_cubie_counts: &[usize],
    puzzle_orbit_definition: &[Orbit],
) -> CycleCombination {
    let mut cycle_combination: Vec<Cycle> = vec![];

    for (r, register) in registers.iter().enumerate() {
        let mut partitions: Vec<Partition> = vec![];

        for (o, orbit) in puzzle_orbit_definition.iter().enumerate() {
            let mut lcm: usize = 1;
            for a in &assignments[r][o] {
                lcm = num_integer::lcm(lcm, *a);
            }

            if orbit.orient_count > 1 {
                lcm *= orbit.orient_count;
                assignments[r][o].push(1);
            }

            partitions.push(Partition {
                name: orbit.name.clone(),
                partition: assignments[r][o].clone(),
                order: lcm,
            });
        }

        cycle_combination.push(Cycle {
            order: register.order,
            partitions,
        });
    }

    let order_product = registers.iter().map(|v| v.order).product();

    CycleCombination {
        used_cubie_counts: cycle_cubie_counts.to_vec(),
        order_product,
        cycles: cycle_combination,
    }
}

fn efficient_cycle_combinations(
    puzzle_orbit_definition: &[Orbit],
    num_registers: usize,
) -> Option<CycleCombination> {
    let mut cycle_cubie_counts: Vec<usize> = vec![0; puzzle_orbit_definition.len()];
    let mut max_orient: Vec<usize> = vec![0; 4];

    for (o, orbit) in puzzle_orbit_definition.iter().enumerate() {
        if orbit.orient_count > 1 {
            max_orient[orbit.orient_count] = orbit.cubie_count - 1;
            cycle_cubie_counts[o] = orbit.cubie_count - 1;
        } else {
            cycle_cubie_counts[o] = orbit.cubie_count;
        }
    }

    let total_cubies: usize = cycle_cubie_counts.iter().sum();
    let cubies_per_register = total_cubies / num_registers;
    let possible_orders: Vec<PrimeCombo> = possible_order_list(
        cubies_per_register,
        *cycle_cubie_counts
            .iter()
            .max()
            .unwrap()
            .min(&cubies_per_register),
        &max_orient,
    );

    for prime_combo in possible_orders {
        println!("Testing Order {}", prime_combo.order);

        let mut unorientable_excess: usize = 0;
        for (v, value) in prime_combo.values.iter().enumerate() {
            if value % 2 == 0 {
                let orientable =
                    (max_orient[2] / 1.max(prime_combo.piece_counts[v])).min(num_registers);
                unorientable_excess +=
                    (num_registers - orientable) * (value - prime_combo.piece_counts[v]);
            } else if value % 3 == 0 {
                let orientable =
                    (max_orient[3] / 1.max(prime_combo.piece_counts[v])).min(num_registers);
                unorientable_excess +=
                    (num_registers - orientable) * (value - prime_combo.piece_counts[v]);
            }
        }

        if unorientable_excess + num_registers * (prime_combo.piece_counts.iter().sum::<usize>())
            > total_cubies
        {
            continue;
        }

        let assignments = cycle_combo_test(
            &vec![prime_combo.clone(); num_registers],
            &cycle_cubie_counts,
            puzzle_orbit_definition,
        );

        if assignments.is_some() {
            return Some(assignments_to_combo(
                &mut assignments.unwrap(),
                &vec![prime_combo.clone(); num_registers],
                &cycle_cubie_counts,
                puzzle_orbit_definition,
            ));
        }
    }

    None
}

fn main() {
    let puzzle_orbit_definition: Vec<Orbit> = vec![
        Orbit {
            name: String::from("corners"),
            cubie_count: 8,
            orient_count: 3,
            orient_sum: 0,
        },
        Orbit {
            name: String::from("edges"),
            cubie_count: 12,
            orient_count: 2,
            orient_sum: 0,
        },
    ];

    let cycle_combos = efficient_cycle_combinations(&puzzle_orbit_definition, 3);

    println!(
        "Highest Equivalent Order: {}",
        cycle_combos.unwrap().cycles[0].order
    );
}
