use std::ptr;

/// Reverse the elements in the slice `perm` from index `s` to index `e`,
/// inclusive.
///
/// # Safety
///
/// `s` and `e` must be valid indices for `perm`, and `s` must be less than or
/// equal to `e`.
unsafe fn reverse_unchecked(perm: &mut [u8], mut s: usize, mut e: usize) {
    while s < e {
        // SAFETY: `perm` is a mutable slice, and `s` and `e` are valid indices
        // for `perm` by the caller
        unsafe {
            swap_unchecked(perm, s, e);
        }
        s += 1;
        e -= 1;
    }
}

/// Swaps the elements at indices `i` and `j` in the slice `perm`.
///
/// # Safety
///
/// `i` and `j` must be valid indices for `perm`
unsafe fn swap_unchecked(perm: &mut [u8], i: usize, j: usize) {
    unsafe {
        ptr::swap(perm.as_mut_ptr().add(i), perm.as_mut_ptr().add(j));
    }
}

/// Use the Pandita algorithm to compute the next lexicographic permutation of a
/// slice. From: <https://web.archive.org/web/20180219061217/https://www.cs.utsa.edu/~wagner/knuth/fasc2b.pdf>
///
/// # Safety
///
/// `perm` cannot be empty and must contain unique elements.
pub unsafe fn pandita1(perm: &mut [u8]) {
    // Benchmarked on a 2025 Mac M4: 175.65ns (test_big) 2.8ns (test_small)

    // SAFETY: The safety of each `unsafe` block is backed by the correctness
    // of the implementation of the given algorithm

    let len = perm.len();
    let mut i = len - 1;
    while unsafe { *perm.get_unchecked(i) < *perm.get_unchecked(i - 1) } {
        if i == 1 {
            return;
        }
        i -= 1;
    }
    for j in (i..len).rev() {
        unsafe {
            if perm.get_unchecked(j) > perm.get_unchecked(i - 1) {
                swap_unchecked(perm, i - 1, j);
                break;
            }
        }
    }
    unsafe {
        reverse_unchecked(perm, i, len - 1);
    }
}

/// Use an alternative implementation of the Pandita algorithm to compute the
/// next lexicographic permutation of a slice. From: <https://www.geeksforgeeks.org/lexicographic-permutations-of-string/>
///
/// # Safety
///
/// `perm` must have length at least two and must contain unique elements.
pub unsafe fn pandita2(perm: &mut [u8]) {
    // Benchmarked on a 2025 Mac M4: 170.49ns (test_big) 2.84ns (test_small)

    // SAFETY: The safety of each `unsafe` block is backed by the correctness
    // of the implementation of the given algorithm

    let len = perm.len();
    let mut i = len - 2;
    while unsafe { perm.get_unchecked(i) >= perm.get_unchecked(i + 1) } {
        if i == 0 {
            return;
        }
        i -= 1;
    }
    let mut j = len - 1;
    while unsafe { perm.get_unchecked(j) <= perm.get_unchecked(i) } {
        j -= 1;
    }
    unsafe {
        swap_unchecked(perm, i, j);
        reverse_unchecked(perm, i + 1, len - 1);
    }
}

/// Use the Ord-Smith algorithm to compute the next lexicographic permutation
/// of a slice. From: <https://web.archive.org/web/20250319143911/https://mathsanew.com/articles/permutations_without_recursion.pdf>
///
/// # Safety
///
/// This function must be used in the exact same manner `test_ordsmith` is used
/// in permutator.rs.
pub unsafe fn ordsmith(perm: &mut [u8], tmp: &mut [u8], s: &mut usize) {
    // Benchmarked on a 2025 Mac M4: 294.25ns (test_big) 2.30ns (test_small)

    // SAFETY: The safety of each `unsafe` block is backed by the correctness
    // of the implementation of the given algorithm

    let len = perm.len();
    loop {
        if usize::from(unsafe { *tmp.get_unchecked(*s) }) < len - *s - 1 {
            unsafe {
                *tmp.get_unchecked_mut(*s) += 1;
                reverse_unchecked(perm, *s + 1, len - 1);
                swap_unchecked(perm, *s, *s + usize::from(*tmp.get_unchecked(*s)));
            }
            *s = len - 2;
            return;
        }
        unsafe {
            *tmp.get_unchecked_mut(*s) = 0;
        }
        if *s == 0 {
            return;
        }
        *s -= 1;
    }
}

/// Use the Permulex algorithm developed by Shrack and Shimrat to compute the
/// next lexicographic permutation of a slice. From: <https://dl.acm.org/doi/pdf/10.1145/367766.368177>
///
/// # Safety
///
/// The function must be used in the exact same manner `test_permulex` is used
/// in permutator.rs.
pub unsafe fn permulex(perm: &mut [u8], len: u8, q: &mut [u8], flag2: &mut bool) {
    // Benchmarked on a 2025 Mac M4: 516.08ns (test_big) 2.48ns (test_small)

    // SAFETY: The safety of each `unsafe` block is backed by the correctness
    // of the implementation of the given algorithm

    if *flag2 {
        unsafe { swap_unchecked(perm, len as usize - 1, len as usize - 2) };
        *flag2 = false;
        return;
    }

    *flag2 = true;

    for i in (0..=len as usize - 3).rev() {
        if unsafe { perm.get_unchecked(i) >= perm.get_unchecked(i + 1) } {
            continue;
        }

        q.fill(u8::MAX);

        for k in i..len as usize {
            unsafe {
                let val = *perm.get_unchecked(k);
                *q.get_unchecked_mut(val as usize) = val;
            }
        }

        for k in (unsafe { *perm.get_unchecked(i) } + 1)..len {
            if *unsafe { q.get_unchecked(k as usize) } == u8::MAX {
                continue;
            }

            unsafe {
                *perm.get_unchecked_mut(i) = k;
                *q.get_unchecked_mut(k as usize) = u8::MAX;
            }
            let mut idx = i + 1;
            for l in 0..len as usize {
                if *unsafe { q.get_unchecked(l) } == u8::MAX {
                    continue;
                }

                unsafe {
                    *perm.get_unchecked_mut(idx) = *q.get_unchecked(l);
                }
                idx += 1;
                if idx >= len as usize {
                    break;
                }
            }

            break;
        }

        break;
    }
}

/// Use a modified version of Knuth's algorithm X to compute the next
/// lexicographic permutation of a slice. From: <https://docs.rs/permutator/latest/src/permutator/lib.rs.html#4949>
///
/// # Safety
///
/// The function must be used in the exact same manner `test_knuthx` is used
/// in permutator.rs.
#[allow(clippy::many_single_char_names)]
pub unsafe fn knuthx(
    perm: &mut [u8],
    a: &mut [u8],
    k: &mut usize,
    l: &mut [u8],
    p: &mut u8,
    q: &mut u8,
    u: &mut [u8],
) {
    // Benchmarked on a 2025 Mac M4: 741.72ns (test_big) 4.44ns (test_small)

    // SAFETY: The safety of each `unsafe` block is backed by the correctness
    // of the implementation of the given algorithm

    let len = perm.len();

    while *k != 0 {
        unsafe {
            *perm.get_unchecked_mut(*k - 1) = *q - 1;
            *a.get_unchecked_mut(*k) = *q;
        }
        if *k == len {
            loop {
                *k -= 1;
                if *k == 0 {
                    return;
                }
                unsafe {
                    *p = *u.get_unchecked(*k);
                    *q = *a.get_unchecked(*k);
                    *l.get_unchecked_mut(*p as usize) = *q;
                }
                *p = *q;
                unsafe {
                    *q = *l.get_unchecked(*p as usize);
                }
                if *q != 0 {
                    return;
                }
            }
        } else {
            unsafe {
                *u.get_unchecked_mut(*k) = *p;
                *l.get_unchecked_mut(*p as usize) = *l.get_unchecked(*q as usize);
            }
            *k += 1;
            *p = 0;
            *q = l[0];
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use super::*;
    use itertools::Itertools;

    const PERM_FIVE: [[u8; 5]; 120] = [
        [0, 1, 2, 3, 4],
        [0, 1, 2, 4, 3],
        [0, 1, 3, 2, 4],
        [0, 1, 3, 4, 2],
        [0, 1, 4, 2, 3],
        [0, 1, 4, 3, 2],
        [0, 2, 1, 3, 4],
        [0, 2, 1, 4, 3],
        [0, 2, 3, 1, 4],
        [0, 2, 3, 4, 1],
        [0, 2, 4, 1, 3],
        [0, 2, 4, 3, 1],
        [0, 3, 1, 2, 4],
        [0, 3, 1, 4, 2],
        [0, 3, 2, 1, 4],
        [0, 3, 2, 4, 1],
        [0, 3, 4, 1, 2],
        [0, 3, 4, 2, 1],
        [0, 4, 1, 2, 3],
        [0, 4, 1, 3, 2],
        [0, 4, 2, 1, 3],
        [0, 4, 2, 3, 1],
        [0, 4, 3, 1, 2],
        [0, 4, 3, 2, 1],
        [1, 0, 2, 3, 4],
        [1, 0, 2, 4, 3],
        [1, 0, 3, 2, 4],
        [1, 0, 3, 4, 2],
        [1, 0, 4, 2, 3],
        [1, 0, 4, 3, 2],
        [1, 2, 0, 3, 4],
        [1, 2, 0, 4, 3],
        [1, 2, 3, 0, 4],
        [1, 2, 3, 4, 0],
        [1, 2, 4, 0, 3],
        [1, 2, 4, 3, 0],
        [1, 3, 0, 2, 4],
        [1, 3, 0, 4, 2],
        [1, 3, 2, 0, 4],
        [1, 3, 2, 4, 0],
        [1, 3, 4, 0, 2],
        [1, 3, 4, 2, 0],
        [1, 4, 0, 2, 3],
        [1, 4, 0, 3, 2],
        [1, 4, 2, 0, 3],
        [1, 4, 2, 3, 0],
        [1, 4, 3, 0, 2],
        [1, 4, 3, 2, 0],
        [2, 0, 1, 3, 4],
        [2, 0, 1, 4, 3],
        [2, 0, 3, 1, 4],
        [2, 0, 3, 4, 1],
        [2, 0, 4, 1, 3],
        [2, 0, 4, 3, 1],
        [2, 1, 0, 3, 4],
        [2, 1, 0, 4, 3],
        [2, 1, 3, 0, 4],
        [2, 1, 3, 4, 0],
        [2, 1, 4, 0, 3],
        [2, 1, 4, 3, 0],
        [2, 3, 0, 1, 4],
        [2, 3, 0, 4, 1],
        [2, 3, 1, 0, 4],
        [2, 3, 1, 4, 0],
        [2, 3, 4, 0, 1],
        [2, 3, 4, 1, 0],
        [2, 4, 0, 1, 3],
        [2, 4, 0, 3, 1],
        [2, 4, 1, 0, 3],
        [2, 4, 1, 3, 0],
        [2, 4, 3, 0, 1],
        [2, 4, 3, 1, 0],
        [3, 0, 1, 2, 4],
        [3, 0, 1, 4, 2],
        [3, 0, 2, 1, 4],
        [3, 0, 2, 4, 1],
        [3, 0, 4, 1, 2],
        [3, 0, 4, 2, 1],
        [3, 1, 0, 2, 4],
        [3, 1, 0, 4, 2],
        [3, 1, 2, 0, 4],
        [3, 1, 2, 4, 0],
        [3, 1, 4, 0, 2],
        [3, 1, 4, 2, 0],
        [3, 2, 0, 1, 4],
        [3, 2, 0, 4, 1],
        [3, 2, 1, 0, 4],
        [3, 2, 1, 4, 0],
        [3, 2, 4, 0, 1],
        [3, 2, 4, 1, 0],
        [3, 4, 0, 1, 2],
        [3, 4, 0, 2, 1],
        [3, 4, 1, 0, 2],
        [3, 4, 1, 2, 0],
        [3, 4, 2, 0, 1],
        [3, 4, 2, 1, 0],
        [4, 0, 1, 2, 3],
        [4, 0, 1, 3, 2],
        [4, 0, 2, 1, 3],
        [4, 0, 2, 3, 1],
        [4, 0, 3, 1, 2],
        [4, 0, 3, 2, 1],
        [4, 1, 0, 2, 3],
        [4, 1, 0, 3, 2],
        [4, 1, 2, 0, 3],
        [4, 1, 2, 3, 0],
        [4, 1, 3, 0, 2],
        [4, 1, 3, 2, 0],
        [4, 2, 0, 1, 3],
        [4, 2, 0, 3, 1],
        [4, 2, 1, 0, 3],
        [4, 2, 1, 3, 0],
        [4, 2, 3, 0, 1],
        [4, 2, 3, 1, 0],
        [4, 3, 0, 1, 2],
        [4, 3, 0, 2, 1],
        [4, 3, 1, 0, 2],
        [4, 3, 1, 2, 0],
        [4, 3, 2, 0, 1],
        [4, 3, 2, 1, 0],
    ];

    const PERM_FOUR: [[u8; 4]; 24] = [
        [0, 1, 2, 3],
        [0, 1, 3, 2],
        [0, 2, 1, 3],
        [0, 2, 3, 1],
        [0, 3, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 2, 3],
        [1, 0, 3, 2],
        [1, 2, 0, 3],
        [1, 2, 3, 0],
        [1, 3, 0, 2],
        [1, 3, 2, 0],
        [2, 0, 1, 3],
        [2, 0, 3, 1],
        [2, 1, 0, 3],
        [2, 1, 3, 0],
        [2, 3, 0, 1],
        [2, 3, 1, 0],
        [3, 0, 1, 2],
        [3, 0, 2, 1],
        [3, 1, 0, 2],
        [3, 1, 2, 0],
        [3, 2, 0, 1],
        [3, 2, 1, 0],
    ];

    #[test]
    fn test_pandita1() {
        let mut len = 5;
        let mut perm = (0..len).collect_vec();
        let mut i = 1;

        while i < PERM_FIVE.len() {
            unsafe { pandita1(&mut perm) };
            assert_eq!(perm, PERM_FIVE[i]);
            i += 1;
        }

        len = 4;
        perm = (0..len).collect_vec();
        i = 1;
        while i < PERM_FOUR.len() {
            unsafe { pandita1(&mut perm) };
            assert_eq!(perm, PERM_FOUR[i]);
            i += 1;
        }
    }

    #[test]
    fn test_pandita2() {
        let mut len = 5;
        let mut perm = (0..len).collect_vec();
        let mut i = 1;

        while i < PERM_FIVE.len() {
            unsafe { pandita2(&mut perm) };
            assert_eq!(perm, PERM_FIVE[i]);
            i += 1;
        }

        len = 4;
        perm = (0..len).collect_vec();
        i = 1;
        while i < PERM_FOUR.len() {
            unsafe { pandita2(&mut perm) };
            assert_eq!(perm, PERM_FOUR[i]);
            i += 1;
        }
    }

    #[test]
    fn test_ordsmith() {
        let mut len = 5;
        let mut perm = (0..len).collect_vec();
        let mut tmp = vec![0; len as usize - 1];
        let mut s = len as usize - 2;
        let mut j = 1;
        while j < PERM_FOUR.len() {
            unsafe { ordsmith(&mut perm, &mut tmp, &mut s) };
            assert_eq!(perm, PERM_FIVE[j]);
            j += 1;
        }

        len = 4;
        perm = (0..len).collect_vec();
        tmp = vec![0; len as usize - 1];
        s = len as usize - 2;
        j = 1;
        while j < PERM_FOUR.len() {
            unsafe { ordsmith(&mut perm, &mut tmp, &mut s) };
            assert_eq!(perm, PERM_FOUR[j]);
            j += 1;
        }
    }

    #[test]
    fn test_permulex() {
        let mut len = 5;

        let mut perm = (0..len).collect_vec();
        let mut q = vec![0; len as usize];
        let mut flag = true;
        let mut i = 1;
        while i < PERM_FIVE.len() {
            unsafe { permulex(&mut perm, len, &mut q, &mut flag) };
            assert_eq!(perm, PERM_FIVE[i]);
            i += 1;
        }

        len = 4;

        perm = (0..len).collect_vec();
        q = vec![0; len as usize];
        flag = true;
        i = 1;
        while i < PERM_FOUR.len() {
            unsafe { permulex(&mut perm, len, &mut q, &mut flag) };
            assert_eq!(perm, PERM_FOUR[i]);
            i += 1;
        }
    }

    #[test]
    fn test_knuthx() {
        #![allow(clippy::many_single_char_names)]
        let mut len = 4;

        let mut perm = vec![0; len as usize];
        let mut a = (0..=len).collect_vec();
        let mut k = 1;
        let mut l = (1..=len).chain(std::iter::once(0)).collect_vec();
        let mut p = 0;
        let mut q = 1;
        let mut u = vec![0; len as usize + 1];

        let mut i = 0;
        while i < PERM_FOUR.len() {
            unsafe { knuthx(&mut perm, &mut a, &mut k, &mut l, &mut p, &mut q, &mut u) };
            assert_eq!(perm, PERM_FOUR[i]);
            i += 1;
        }

        len = 5;
        perm = vec![0; len as usize];
        a = (0..=len).collect_vec();
        k = 1;
        l = (1..=len).collect_vec();
        l.push(0);
        p = 0;
        q = 1;
        u = vec![0; len as usize + 1];

        i = 0;
        while i < PERM_FIVE.len() {
            unsafe { knuthx(&mut perm, &mut a, &mut k, &mut l, &mut p, &mut q, &mut u) };
            assert_eq!(perm, PERM_FIVE[i]);
            i += 1;
        }
    }

    #[bench]
    fn bench_pandita1_small(b: &mut test::Bencher) {
        let len = 12;
        let mut perm = (0..len).collect_vec().into_boxed_slice();
        b.iter(|| unsafe {
            pandita1(test::black_box(&mut perm));
        });
    }

    #[bench]
    fn bench_pandita2_small(b: &mut test::Bencher) {
        let len = 12;
        let mut perm = (0..len).collect_vec().into_boxed_slice();
        b.iter(|| unsafe {
            pandita2(test::black_box(&mut perm));
        });
    }

    #[bench]
    fn bench_ordsmith_small(b: &mut test::Bencher) {
        let len = 12;
        let mut perm = (0..len).collect_vec().into_boxed_slice();
        let mut tmp = vec![0; len as usize - 1].into_boxed_slice();
        let mut s = len as usize - 2;
        b.iter(|| unsafe {
            ordsmith(
                test::black_box(&mut perm),
                test::black_box(&mut tmp),
                test::black_box(&mut s),
            );
        });
    }

    #[bench]
    fn bench_permulex_small(b: &mut test::Bencher) {
        let len = 12;
        let mut perm = (0..len).collect_vec();
        let mut q = vec![0; len as usize];
        let mut flag = true;
        b.iter(|| unsafe {
            permulex(
                test::black_box(&mut perm),
                test::black_box(len),
                test::black_box(&mut q),
                test::black_box(&mut flag),
            );
        });
    }

    #[bench]
    fn bench_knuthx_small(b: &mut test::Bencher) {
        #![allow(clippy::many_single_char_names)]
        let len = 12;

        let mut perm = vec![0; len as usize];
        let mut a = (0..=len).collect_vec().into_boxed_slice();
        let mut k = 1;
        let mut l = (1..=len)
            .chain(std::iter::once(0))
            .collect_vec()
            .into_boxed_slice();
        let mut p = 0;
        let mut q = 1;
        let mut u = vec![0; len as usize + 1];

        b.iter(|| unsafe {
            knuthx(&mut perm, &mut a, &mut k, &mut l, &mut p, &mut q, &mut u);
        });
    }

    #[bench]
    fn bench_pandita1_big(b: &mut test::Bencher) {
        let len = 5;
        let mut perm = vec![0; len as usize].into_boxed_slice();
        b.iter(|| {
            for i in 0..len {
                perm[i as usize] = i;
            }
            let mut i = 1;
            while i < test::black_box(PERM_FIVE.len()) {
                unsafe { pandita1(test::black_box(&mut perm)) };
                i += 1;
            }
        });
    }

    #[bench]
    fn bench_pandita2_big(b: &mut test::Bencher) {
        let len = 5;
        let mut perm = vec![0; len as usize].into_boxed_slice();
        b.iter(|| {
            for i in 0..len {
                perm[i as usize] = i;
            }
            let mut i = 1;
            while i < test::black_box(PERM_FIVE.len()) {
                unsafe { pandita2(test::black_box(&mut perm)) };
                i += 1;
            }
        });
    }

    #[bench]
    fn bench_ordsmith_big(b: &mut test::Bencher) {
        let len = 5;
        let mut perm = vec![0_u8; len as usize].into_boxed_slice();
        let mut tmp = vec![0; len as usize].into_boxed_slice();
        b.iter(|| {
            for i in 0..len {
                perm[i as usize] = i;
            }
            let mut s = test::black_box(len) as usize - 2;
            let mut i = 1;
            while i < test::black_box(PERM_FIVE.len()) {
                unsafe {
                    ordsmith(
                        test::black_box(&mut perm),
                        test::black_box(&mut tmp),
                        test::black_box(&mut s),
                    );
                };
                i += 1;
            }
        });
    }

    #[bench]
    fn bench_permulex_big(b: &mut test::Bencher) {
        let len = 5;
        let mut perm = vec![0_u8; len as usize].into_boxed_slice();
        let mut q = vec![0; len as usize].into_boxed_slice();
        b.iter(|| {
            for i in 0..len {
                perm[i as usize] = i;
            }
            q.fill(0);
            let mut flag = true;
            let mut i = 1;
            while i < test::black_box(PERM_FIVE.len()) {
                unsafe {
                    permulex(
                        test::black_box(&mut perm),
                        test::black_box(len),
                        test::black_box(&mut q),
                        test::black_box(&mut flag),
                    );
                };
                i += 1;
            }
        });
    }

    #[bench]
    fn bench_knuthx_big(b: &mut test::Bencher) {
        #![allow(clippy::many_single_char_names)]
        let len = 5;

        let mut perm = vec![0; len as usize];
        let mut a = (0..=len).collect_vec();
        let mut k = 1;
        let mut l = (1..=len).chain(std::iter::once(0)).collect_vec();
        let mut p = 0;
        let mut q = 1;
        let mut u = vec![0; len as usize + 1];

        b.iter(|| {
            perm.fill(0);
            for i in 0..=len {
                a[i as usize] = i;
            }
            k = 1;
            for i in 0..len {
                l[i as usize] = i + 1;
            }
            l[len as usize] = 0;
            p = 0;
            q = 1;
            u.fill(0);

            let mut i = 0;
            while i < test::black_box(PERM_FIVE.len()) {
                unsafe {
                    knuthx(
                        test::black_box(&mut perm),
                        test::black_box(&mut a),
                        test::black_box(&mut k),
                        test::black_box(&mut l),
                        test::black_box(&mut p),
                        test::black_box(&mut q),
                        test::black_box(&mut u),
                    );
                }
                i += 1;
            }
        });
    }
}
