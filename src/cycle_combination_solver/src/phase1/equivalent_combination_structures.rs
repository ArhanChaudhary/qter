


struct PrimePower {
    value: usize,
    pieces: usize
}
struct OrderIteration {
    index: isize,
    piece_count: usize,
    product: usize,
    powers: Vec<usize>,
    pieces: Vec<usize>
}

struct ComboIteration {
    r: usize,
    p: usize,
    orbit_sums: Vec<usize>,
    assignments: Vec<Vec<Vec<usize>>>,
    available_pieces: usize
}

struct PrimeCombo {
    order: usize,
    values: Vec<usize>,
    piece_total: usize,
    piece_counts: Vec<usize>
}

struct Orbit {
    name: String,
    cubie_count: usize,
    orient_count: usize,
    orient_sum: usize
}

struct Partition {
    name: String,
    partition: Vec<usize>,
    order: usize
}

struct Cycle {
    order: usize,
    partitions: Vec<Partition>
}

struct CycleCombination {
    used_cubie_counts: Vec<usize>,
    order_product: usize,
    cycles: Vec<Cycle>
}


fn prime_powers_below_n(n: &usize, max_orient: &Vec<usize>) -> Vec<Vec<PrimePower>>{

    let mut primes: Vec<usize> = vec![2];

    for possible_prime in (3..n+1).step_by(2){
        
        let mut is_prime = true;
        for p in primes.iter(){
            if p.pow(2) > possible_prime {
                break;
            }

            if possible_prime % p == 0{
                is_prime = false;
                break;
            }
        }

        if !is_prime{
            continue;
        }

        primes.push(possible_prime);
    }

    let mut prime_powers = vec![];

    for p in primes.iter().enumerate(){
        
        let mut orient: usize = 1;
        let mut piece_check = p.1.pow(2);
        if max_orient.len() > *p.1 && max_orient[*p.1] > 0{
            orient = *p.1;
            piece_check = *p.1;
            prime_powers.push(vec![
                PrimePower{value: 1, pieces: 0},
                PrimePower{value: *p.1, pieces: 0}
                ]
            )
        } else {
            prime_powers.push(vec![
                PrimePower{value: 1, pieces: 0},
                PrimePower{value: *p.1, pieces: *p.1}
                ]
            )
        }


        while piece_check <= *n{
            prime_powers[p.0].push(
                PrimePower{value: orient * piece_check, pieces: piece_check}
            );

            piece_check *= *p.1;
            if orient > 1 && piece_check > max_orient[*p.1]{
                piece_check *= orient;
                orient = 1;
            }

        }

    }

    return prime_powers;
}


fn possible_order_list(
    total_pieces: &usize,
    partition_max: &usize,
    max_orient: &Vec<usize>
    ) -> Vec<PrimeCombo>{

    let prime_powers = prime_powers_below_n(partition_max,max_orient);

    let mut paths = vec![];
    let mut stack: Vec<OrderIteration> = vec![OrderIteration{
        index: (prime_powers.len() - 1) as isize,
        piece_count: 0,
        product: 1,
        powers: vec![],
        pieces: vec![]
    }];

    while stack.len() > 0{

        let s = stack.pop().unwrap();
        if s.index == -1 || prime_powers[s.index as usize][1].pieces + s.piece_count > *total_pieces{
            paths.push(PrimeCombo{
                order: s.product,
                values: s.powers.clone(),
                piece_total: s.pieces.clone().into_iter().sum(),
                piece_counts: s.pieces.clone()
            });
            continue;
        }

        for p in prime_powers[s.index as usize].iter(){
            let mut new_pieces = s.piece_count + p.pieces;
            if p.pieces > 0 && p.pieces % 2 == 0{ // TODO this should not happen on 4x4
                new_pieces += 2;
            }

            if new_pieces <= *total_pieces{

                stack.push(OrderIteration{
                    index: s.index - 1,
                    piece_count: new_pieces,
                    product: s.product,
                    powers: s.powers.clone(),
                    pieces: s.pieces.clone()
                });
                
                if p.value > 1{
                    let s_last = stack.len() - 1;
                    stack[s_last].product *= p.value;
                    stack[s_last].powers.push(p.value);
                    stack[s_last].pieces.push(p.pieces);
                }

            }
        }
    }

    
    paths.sort_by( |a: &PrimeCombo, b: &PrimeCombo| a.order.partial_cmp(&b.order).unwrap());

    return paths;

}


fn cycle_combo_test(
    registers: &Vec<PrimeCombo>,
    cycle_cubie_counts: &Vec<usize>,
    puzzle_orbit_definition: &Vec<Orbit>
    ) -> Vec<Vec<Vec<usize>>>{

    let mut stack: Vec<ComboIteration> = vec![ComboIteration{
        r: 0,
        p: 0,
        orbit_sums: vec![0; cycle_cubie_counts.len()],
        assignments: vec![vec![vec![]; cycle_cubie_counts.len()]; registers.len()],
        available_pieces: cycle_cubie_counts.iter().sum() //TODO this is wrong
    }];

    let mut loops: usize = 0;
    while stack.len() > 0{

        loops += 1;
        if loops > 1000{
            return vec![vec![vec![]]];
        }

        let mut s  = stack.pop().unwrap();

        let mut seen = vec![];
        while s.p == registers[s.r].values.len(){
            s.p = 0;
            s.r += 1;
            if s.r == registers.len(){
                break;
            }
        }

        if s.r == registers.len(){
            return s.assignments;
        } //TODO no duplicates

        for orbit in puzzle_orbit_definition.iter().enumerate(){

            if orbit.1.orient_count == 1{
                if seen.contains(&cycle_cubie_counts[orbit.0]) {
                    continue;
                } else {
                    seen.push(cycle_cubie_counts[orbit.0]);
                }
            }

            let mut new_cycle: usize;
            let mut new_available: usize;
            if orbit.1.orient_count > 1 && registers[s.r].values[s.p] % orbit.1.orient_count == 0{
                new_cycle = registers[s.r].piece_counts[s.p];
                new_available = s.available_pieces;
            } else if registers[s.r].values[s.p] - registers[s.r].piece_counts[s.p] <= s.available_pieces{
                new_cycle = registers[s.r].values[s.p];
                new_available = s.available_pieces - registers[s.r].values[s.p] + registers[s.r].piece_counts[s.p];
            } else {
                continue;
            }

            if new_cycle == 0 && s.assignments[s.r][orbit.0].len() == 0{
                if s.available_pieces == 0{
                    continue;
                }
                new_cycle = 1;
            }

            let mut parity: usize = 0;
            if new_cycle % 2 == 0 && new_cycle > 0{
                parity = 2;
            }

            if new_cycle + parity + s.orbit_sums[orbit.0] <= cycle_cubie_counts[orbit.0]{
                stack.push(ComboIteration{
                    r: s.r,
                    p: s.p + 1,
                    orbit_sums: s.orbit_sums.clone(),
                    assignments: s.assignments.clone(),
                    available_pieces: new_available
                });

                if new_cycle > 0 {
                    let last = stack.len() - 1;
                    stack[last].orbit_sums[orbit.0] += new_cycle;
                    stack[last].assignments[s.r][orbit.0].push(new_cycle);
                    if parity > 0{
                        stack[last].orbit_sums[orbit.0] += 2;
                        stack[last].assignments[s.r][orbit.0].push(2);
                    }
                }
            }
        }
    }

    vec![vec![vec![]]]
}

fn assignments_to_combo(
    assignments: &Vec<Vec<Vec<usize>>>,
    registers: &Vec<PrimeCombo>,
    cycle_cubie_counts: &Vec<usize>,
    puzzle_orbit_definition: &Vec<Orbit>
){
    
    let mut cycle_combination: Vec<Cycle> = vec![];

    for r in registers.iter().enumerate(){
        let mut partitions: Vec<Partition> = vec![];

        for orbit in puzzle_orbit_definition.iter().enumerate(){

            let mut lcm: usize = 1;
            for a in assignments[r.0][orbit.0]{
                lcm = num_integer::lcm(l, a);
            }


            if orbit.1.orient_count > 1{
                lcm *= orbit.1.orient_count;
                assignments[r.0][orbit.0].push(1);
            }

            partitions.push(Partition{
                name: orbit.1.name,
                partition: assignments[r.0][orbit.0],
                order: lcm
            });
        }

        cycle_combination.push(Cycle { order: r.1.order, partitions: partitions});
    }

    return CycleCombination{     
        used_cubie_counts: cycle_cubie_counts,
        order_product: registers.iter().product(),
        cycles: cycle_combination
    };    

}

fn main() {

    let n = 10;
    let partition_max = 4;
    let max_orient = vec![0,0,0,4];
    possible_order_list(&n,&partition_max, &max_orient);
}
