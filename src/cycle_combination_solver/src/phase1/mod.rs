use puzzle_geometry::ksolve::{self, KSolveSet};
use qter_core::{Int, U};
use std::num::NonZero;

struct PrimePower {
    value: u16,
    min_pieces: u16,
}
struct OrderIteration {
    index: usize,
    piece_count: u16,
    product: Int<U>,
    powers: Vec<u16>,
    min_pieces: Vec<u16>,
}

struct ComboIteration {
    register: usize,
    power: usize,
    orbit_sums: Vec<u16>,
    assignments: Vec<Assignment>,
    available_pieces: u16,
}

type Assignment = Vec<Vec<u16>>;

#[derive(Clone)]
struct PossibleOrder {
    // this is a candidate order
    order: Int<U>,
    prime_powers: Vec<u16>,
    min_piece_counts: Vec<u16>,
}

struct Partition {
    name: String,
    partition: Vec<u16>,
    order: Int<U>,
}

struct Cycle {
    order: Int<U>,
    partitions: Vec<Partition>,
}

struct CycleCombination {
    used_cubie_counts: Vec<u16>,
    order_product: Int<U>,
    cycles: Vec<Cycle>,
}

// return a 2D list of prime powers below n. The first index is the prime, the second is the power of that prime
fn prime_powers_below_n(n: u16, orientable_pieces: &[u16]) -> Vec<Vec<PrimePower>> {
    let mut primes: Vec<u16> = vec![2];

    // find all primes below n
    for possible_prime in (3..n + 1).step_by(2) {
        let mut is_prime = true;
        for p in primes.iter() {
            if p * p > possible_prime {
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

    //for each prime, find all of its powers and the minimum pieces needed
    for (p, prime) in primes.iter().enumerate() {
        let mut orient_multiplier: u16 = 1;
        let mut piece_check;
        // handle if there is an orbit with an orient_count of the current prime
        if orientable_pieces.len() > *prime as usize && orientable_pieces[*prime as usize] > 0 {
            orient_multiplier = *prime;
            prime_powers.push(vec![
                PrimePower {
                    value: 1,
                    min_pieces: 0, // excluding this prime uses no pieces
                },
                PrimePower {
                    value: *prime,
                    min_pieces: 0, // the minimum pieces for this prime is 0 since we can use a cycle of different prime length and orient
                },
            ]);
            piece_check = *prime;
        } else {
            prime_powers.push(vec![
                PrimePower {
                    value: 1,
                    min_pieces: 0,
                },
                PrimePower {
                    value: *prime,
                    min_pieces: *prime,
                },
            ]);
            piece_check = prime.pow(2);
        }

        // increase powers of this prime until one doesn't fit
        while piece_check <= n {
            prime_powers[p].push(PrimePower {
                value: orient_multiplier * piece_check,
                min_pieces: piece_check,
            });
            piece_check *= *prime;

            // if the power exceeds the size of orientable orbit, remove the multiplier
            if orient_multiplier > 1 && piece_check > orientable_pieces[*prime as usize] {
                piece_check *= orient_multiplier;
                orient_multiplier = 1;
            }
        }
    }

    prime_powers
}

// get a list of all possible orders to fit within a given number of pieces and partitions
fn possible_order_list(
    total_pieces: u16,
    partition_max: u16,
    orientable_pieces: &[u16],
) -> Vec<PossibleOrder> {
    // get list of prime powers that fit within the largest partition
    let prime_powers = prime_powers_below_n(partition_max, orientable_pieces);

    let mut paths = vec![];
    // create a stack to handle recursive
    let mut stack: Vec<OrderIteration> = vec![OrderIteration {
        index: 0,
        piece_count: 0,
        product: Int::<U>::from(1_u16),
        powers: vec![],
        min_pieces: vec![],
    }];

    // loop through the prime powers, taking all combinations that will fit within total_pieces
    while let Some(s) = stack.pop() {
        // if all primes have been added or there's no room for the next prime, log this order
        if s.index == prime_powers.len()
            || prime_powers[s.index][1].min_pieces + s.piece_count > total_pieces
        {
            paths.push(PossibleOrder {
                order: s.product,
                prime_powers: s.powers.clone(),
                min_piece_counts: s.min_pieces.clone(),
            });
            continue;
        }

        // try adding all powers of the current prime
        for p in prime_powers[s.index].iter() {
            // the new piece count will add min_pieces for the current power, plus two if parity needs handling
            let new_piece_count = s.piece_count
                + p.min_pieces
                + if p.min_pieces > 0 && p.min_pieces % 2 == 0 {
                    2
                } else {
                    0
                }; // TODO this should not happen on 4x4

            // if the new prime power fits on the puzzle, add to the stack
            if new_piece_count <= total_pieces {
                stack.push(OrderIteration {
                    index: s.index + 1,
                    piece_count: new_piece_count,
                    product: s.product,
                    powers: s.powers.clone(),
                    min_pieces: s.min_pieces.clone(),
                });

                if p.value > 1 {
                    let s_last = stack.len() - 1;
                    stack[s_last].product *= Int::<U>::from(p.value);
                    stack[s_last].powers.push(p.value);
                    stack[s_last].min_pieces.push(p.min_pieces);
                }
            }
        }
    }

    paths.sort_by(|a: &PossibleOrder, b: &PossibleOrder| b.order.partial_cmp(&a.order).unwrap());

    paths
}

// given some order, test if it will fit on the puzzle
fn possible_order_test(
    registers: &[PossibleOrder],
    cycle_cubie_counts: &[u16],
    puzzle: &[KSolveSet],
    available_pieces: u16,
) -> Option<Vec<Assignment>> {
    // create a stack to recursively add cycles for prime powers from each register
    let mut stack: Vec<ComboIteration> = vec![ComboIteration {
        register: 0,                                   // current register to add
        power: registers[0].prime_powers.len(), // current prime power index to add (reversed)
        orbit_sums: vec![0; cycle_cubie_counts.len()], // pieces used in each orbit
        assignments: vec![vec![vec![]; cycle_cubie_counts.len()]; registers.len()],
        available_pieces, // extra pieces beyond the minimum
    }];

    let mut loops: u16 = 0;
    while let Some(mut s) = stack.pop() {
        loops += 1;
        if loops > 1000 {
            return None; // a fit is usually found quickly, so quit if the search takes a while
        }

        let mut seen = vec![]; // this is used to detect duplicates

        // if we've added the last prime power for this register, move to the next register
        if s.power == 0 {
            s.power = registers[0].prime_powers.len() - 1;
            s.register += 1;
            // if that was the last register, we found a fit! return it.
            if s.register == registers.len() {
                return Some(s.assignments);
            }
        } else {
            s.power -= 1;
        }

        // try adding the current prime power to each orbit
        for (o, orbit) in puzzle.iter().enumerate() {
            // orbits with no orientation and the same piece count act the same. we should only check the first one
            // continue if this is a duplicate of an orbit that was already checked.
            if orbit.orientation_count().get() == 1 {
                if seen.contains(&cycle_cubie_counts[o]) {
                    continue;
                } else {
                    seen.push(cycle_cubie_counts[o]);
                }
            }

            let mut new_cycle: u16;
            let mut new_available: u16;
            // if this orbit orients using the same prime as the power, add a cycle
            if orbit.orientation_count().get() > 1
                && registers[s.register].prime_powers[s.power]
                    % orbit.orientation_count().get() as u16
                    == 0
            {
                new_cycle = registers[s.register].min_piece_counts[s.power];
                new_available = s.available_pieces;
            }
            // otherwise, we get no orientation multiplier, so the cycle will use the same number of pieces as the power itself
            // if there are enough available pieces to make this happen, add a cycle
            else if registers[s.register].prime_powers[s.power]
                - registers[s.register].min_piece_counts[s.power]
                <= s.available_pieces
            {
                new_cycle = registers[s.register].prime_powers[s.power];
                new_available = s.available_pieces
                    + registers[s.register].min_piece_counts[s.power]
                    - registers[s.register].prime_powers[s.power];
            }
            // but if there are not enough available, continue
            else {
                continue;
            }

            // we assume 0 min_pieces for a prime cycle if there is an orbit with that prime as its orient_count
            // but that requires the orbit to have a cycle of length of a different prime
            // if there is no cycle in this register, we need to use a piece for this.
            if new_cycle == 0 && s.assignments[s.register][o].is_empty() {
                if s.available_pieces == 0 {
                    continue;
                }
                new_cycle = 1;
                new_available -= 1;
                // TODO add some kind of tracker here to make sure orbits aren't doubling
            }

            // assume that every even cycle needs a parity to go with it. TODO make this assumption more efficient.
            let parity: u16 = if new_cycle % 2 == 0 && new_cycle > 0 {
                2
            } else {
                0
            };
            if parity > new_available {
                continue;
            }

            // if there is room for the new cycle in this orbit, add it and push to stack
            if new_cycle + parity + s.orbit_sums[o] <= cycle_cubie_counts[o] {
                stack.push(ComboIteration {
                    register: s.register,
                    power: s.power,
                    orbit_sums: s.orbit_sums.clone(),
                    assignments: s.assignments.clone(),
                    available_pieces: new_available - parity,
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

// once an order is found that fits on the cube, process into an output format
fn assignments_to_combo(
    assignments: &mut [Vec<Vec<u16>>],
    registers: &[PossibleOrder],
    cycle_cubie_counts: &[u16],
    puzzle: &[KSolveSet],
) -> CycleCombination {
    let mut cycle_combination: Vec<Cycle> = vec![];

    for (r, register) in registers.iter().enumerate() {
        let mut partitions: Vec<Partition> = vec![];

        for (o, orbit) in puzzle.iter().enumerate() {
            let mut lcm: Int<U> = Int::<U>::from(1_u16);
            for a in &assignments[r][o] {
                lcm = qter_core::discrete_math::lcm(lcm, Int::<U>::from(*a));
            }

            if orbit.orientation_count().get() > 1 {
                lcm *= Int::<U>::from(orbit.orientation_count().get());
                assignments[r][o].push(1);
            }

            partitions.push(Partition {
                name: orbit.name().to_string(),
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

// this is the main function. it returns a 'near optimal' combination such that all registers have equivalent order
// it may not be the most optimal, since there are some assumptions made to help efficiency
fn optimal_equivalent_combination(
    puzzle: &[KSolveSet],
    num_registers: u16,
) -> Option<CycleCombination> {
    let mut cycle_cubie_counts: Vec<u16> = vec![0; puzzle.len()]; //the count of pieces in each orbit
    let mut orientable_pieces: Vec<u16> = vec![0; 4]; // the kth index stores the number of pieces in an orbit with orient_count k

    // get number of pieces in each orbit. if the orbit pieces can orient, set a shared piece aside to allow free orientation.
    for (o, orbit) in puzzle.iter().enumerate() {
        if orbit.orientation_count().get() > 1 {
            orientable_pieces[orbit.orientation_count().get() as usize] =
                orbit.piece_count().get() - 1;
            cycle_cubie_counts[o] = orbit.piece_count().get() - 1;
        } else {
            cycle_cubie_counts[o] = orbit.piece_count().get();
        }
    }

    let total_cubies: u16 = cycle_cubie_counts.iter().sum();
    let cubies_per_register = total_cubies / num_registers;

    // get a list of all orders that would fit within a cubies_per_register amount of pieces
    let possible_orders: Vec<PossibleOrder> = possible_order_list(
        cubies_per_register,
        *cycle_cubie_counts
            .iter()
            .max()
            .unwrap()
            .min(&cubies_per_register),
        &orientable_pieces,
    );

    // check the possible orders, descending, until one is found that fits
    for possible_order in possible_orders {
        println!("Testing Order {}", possible_order.order);

        // by default, prime_combo.piece_counts assumes all orientation efficiencies can be made
        // here we check if they can actually fit, or if they must be handled by non-orienting pieces
        let mut unorientable_excess: u16 = 0;
        for (p, prime_power) in possible_order.prime_powers.iter().enumerate() {
            if prime_power % 2 == 0 {
                // find the amount of registers that can't be oriented
                let orientable_registers = (orientable_pieces[2]
                    / 1.max(possible_order.min_piece_counts[p]))
                .min(num_registers);
                // each unorientable register will use 'value' pieces instead of 'prime_combo.piece_counts[v]' pieces
                // so we need to account for that difference
                unorientable_excess += (num_registers - orientable_registers)
                    * (prime_power - possible_order.min_piece_counts[p]);
            } else if prime_power % 3 == 0 {
                let orientable_registers = (orientable_pieces[3]
                    / 1.max(possible_order.min_piece_counts[p]))
                .min(num_registers);
                unorientable_excess += (num_registers - orientable_registers)
                    * (prime_power - possible_order.min_piece_counts[p]);
            }
        }

        let available_pieces =
            total_cubies - num_registers * (possible_order.min_piece_counts.iter().sum::<u16>());
        // if the excess exceeds the total number of cubies, the order won't fit so we skip to the next
        if unorientable_excess > available_pieces {
            continue;
        }

        let assignments = possible_order_test(
            &vec![possible_order.clone(); num_registers as usize],
            &cycle_cubie_counts,
            puzzle,
            available_pieces,
        );

        if assignments.is_some() {
            return Some(assignments_to_combo(
                &mut assignments.unwrap(),
                &vec![possible_order.clone(); num_registers as usize],
                &cycle_cubie_counts,
                puzzle,
            ));
        }
    }

    None
}

fn main() {
    let puzzle = ksolve::KSolve::sets(&ksolve::KPUZZLE_3X3);
    let cycle_combos: Option<CycleCombination> = optimal_equivalent_combination(puzzle, 3);

    println!(
        "Highest Equivalent Order: {}",
        cycle_combos.unwrap().cycles[0].order
    );
}

/*
#[test]
fn test_table_encoding() {
    main();
}
*/
