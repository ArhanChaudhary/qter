use integer_partitions::Partitions;
use memoize::memoize;
use std::time::Instant;

enum OrientationFactor {
    One,
    GtOne {
        factor: u32,
        constaint: OrientationSumConstraint,
    },
}

enum OrientationSumConstraint {
    Zero,
    None,
}

#[derive(Debug)]
struct ParetoElement {
    vec: Vec<i32>,
    hello: Option<i32>,
}

impl pareto_front::Dominate for ParetoElement {
    fn dominate(&self, other: &Self) -> bool {
        let mut different = false;
        for (a, b) in self.vec.iter().zip(other.vec.iter()) {
            match a.cmp(b) {
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Greater => different = true,
                std::cmp::Ordering::Equal => (),
            }
        }
        if different {
            true
        } else {
            self.hello == other.hello
        }
    }
}

#[memoize]
fn memoized_integer_partitions(n: usize) -> Vec<Vec<usize>> {
    let mut p = Partitions::new(n);
    let mut partitions = Vec::new();

    while let Some(x) = p.next() {
        partitions.push(x.to_vec());
    }

    partitions
}

fn main() {
    let tests: &[usize] = &[
        1, 1, 2, 3, 5, 7, 11, 15, 22, 30, 42, 56, 77, 101, 135, 176, 231, 297, 385, 490, 627, 792,
        1002, 1255, 1575, 1958, 2436, 3010, 3718, 4565, 5604, 6842, 8349, 10143, 12310, 14883,
        17977, 21637, 26015, 31185, 37338, 44583, 53174, 63261, 75175, 89134, 105558, 124754,
        147273, 173525,
    ];

    let now = Instant::now();
    for (i, &n) in tests.iter().enumerate() {
        let mut p = Partitions::new(i);
        let mut c = 0;

        while let Some(x) = p.next() {
            let sum: usize = x.iter().cloned().sum();
            assert_eq!(sum, i);
            c += 1;
        }

        assert_eq!(c, n);
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    let now = Instant::now();
    for (i, &n) in tests.iter().enumerate() {
        let mut p = Partitions::new(i);
        let mut c = 0;

        while let Some(x) = p.next() {
            let sum: usize = x.iter().cloned().sum();
            assert_eq!(sum, i);
            c += 1;
        }

        assert_eq!(c, n);
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
