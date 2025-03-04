#![cfg_attr(not(avx2), allow(dead_code, unused_variables))]

use super::common::Cube3Interface;
#[cfg(target_arch = "x86")]
use core::arch::x86;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as x86;
use std::{fmt, hash::Hash, simd::u8x32};

#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Clone, Hash)]
#[repr(transparent)]
pub struct Cube3(u8x32);

impl PartialEq for Cube3 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        #[cfg(avx2)]
        extern "vectorcall" fn eq_vectorcall(a: &Cube3, b: &Cube3) -> bool {
            a.0 == b.0
        }
        #[cfg(not(avx2))]
        fn eq_vectorcall(a: &Cube3, b: &Cube3) -> bool {
            unimplemented!()
        }
        eq_vectorcall(self, other)
    }
}

impl fmt::Debug for Cube3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ep = [0; 16];
        let mut eo = [0; 16];
        let mut cp = [0; 16];
        let mut co = [0; 16];

        for i in 0..16 {
            ep[i] = self.0[i] & 0b1111;
            eo[i] = self.0[i] >> 4;
        }

        for i in 16..32 {
            cp[i] = self.0[i] & 0b111;
            co[i] = self.0[i] >> 4;
        }

        f.debug_struct("Cube3")
            .field("ep", &ep)
            .field("eo", &eo)
            .field("cp", &cp)
            .field("co", &co)
            .finish()
    }
}

const PERM_MASK: u8x32 = u8x32::splat(0b1111);

impl Cube3Interface for Cube3 {
    fn from_sorted_transformations(sorted_transformations: &[Vec<(u8, u8)>]) -> Self {
        let corners_transformation = &sorted_transformations[0];
        let edges_transformation = &sorted_transformations[1];

        let mut cube = u8x32::splat(15);

        for i in 0..12 {
            let (perm, ori) = edges_transformation[i];
            cube[i] = perm | (ori << 4);
        }

        for i in 16..24 {
            let (perm, ori) = corners_transformation[i - 16];
            cube[i] = perm | (ori << 4);
        }

        Cube3(cube)
    }

    #[inline(always)]
    fn replace_compose(&mut self, a: &Self, b: &Self) {
        #[cfg(avx2)]
        extern "vectorcall" fn replace_compose_vectorcall(dst: &mut Cube3, a: &Cube3, b: &Cube3) {
            use std::simd::cmp::SimdOrd;

            const ORI_MASK: u8x32 = u8x32::splat(0b11_0000);
            const ORI_CARRY: u8x32 = u8x32::from_array([
                0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
                0x20, 0x20, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
                0x30, 0x30, 0x30, 0x30,
            ]);

            // SAFETY: a and b are well defined. Testing has shown this to be
            // safe.
            let mut composed: u8x32 =
                unsafe { x86::_mm256_shuffle_epi8(a.0.into(), b.0.into()).into() };
            composed += b.0 & ORI_MASK;
            composed = composed.simd_min(composed - ORI_CARRY);

            dst.0 = composed;
        }
        #[cfg(not(avx2))]
        fn replace_compose_vectorcall(_dst: &mut Cube3, _a: &Cube3, _b: &Cube3) {
            unimplemented!()
        }
        replace_compose_vectorcall(self, a, b);
    }

    #[inline(always)]
    fn replace_inverse(&mut self, a: &Self) {
        #[cfg(avx2)]
        extern "vectorcall" fn replace_inverse_vectorcall(dst: &mut Cube3, a: &Cube3) {
            use std::simd::cmp::SimdOrd;

            const ORI_CARRY_INVERSE: u8x32 = u8x32::from_array([
                0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
                0x10, 0x10, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
                0x30, 0x30, 0x30, 0x30,
            ]);

            let perm = a.0 & PERM_MASK;
            let mut ori = a.0 ^ perm;
            let perm = perm.into();

            // See simd8and16 for what this is
            // SAFETY: all arguments are well defined. Testing has shown this to
            // be safe.
            let inverse_perm: u8x32 = unsafe {
                let mut pow_3 = x86::_mm256_shuffle_epi8(perm, perm);
                pow_3 = x86::_mm256_shuffle_epi8(pow_3, perm);
                let mut inverse_perm = x86::_mm256_shuffle_epi8(pow_3, pow_3);
                inverse_perm = x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm);
                inverse_perm = x86::_mm256_shuffle_epi8(
                    x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm),
                    pow_3,
                );
                inverse_perm = x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm);
                inverse_perm = x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm);
                inverse_perm = x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm);
                inverse_perm = x86::_mm256_shuffle_epi8(
                    x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm),
                    perm,
                );
                inverse_perm = x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm);
                inverse_perm = x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm);
                inverse_perm = x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm);
                inverse_perm = x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm);
                inverse_perm = x86::_mm256_shuffle_epi8(
                    x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm),
                    pow_3,
                );
                x86::_mm256_shuffle_epi8(x86::_mm256_shuffle_epi8(inverse_perm, inverse_perm), perm)
                    .into()
            };
            ori += ori;
            ori = ori.simd_min(ori - ORI_CARRY_INVERSE);
            // SAFETY: ori and inverse_perm are well defined. Testing has shown
            // this to be safe.
            ori = unsafe { x86::_mm256_shuffle_epi8(ori.into(), inverse_perm.into()).into() };
            *dst = Cube3(inverse_perm | ori);
        }
        #[cfg(not(avx2))]
        fn replace_inverse_vectorcall(_dst: &mut Cube3, _a: &Cube3) {
            unimplemented!()
        }
        replace_inverse_vectorcall(self, a);
    }

    fn ep_eo_cp_co(
        &self,
        ep: &mut [u8; 16],
        eo: &mut [u8; 16],
        cp: &mut [u8; 8],
        co: &mut [u8; 8],
    ) {
        // TODO: use simd swizzling to make faster

        let perm = (self.0 & PERM_MASK).to_array();
        let ori = (self.0 >> 4).to_array();
        ep.copy_from_slice(&perm[..16]);
        cp.copy_from_slice(&perm[16..24]);
        eo.copy_from_slice(&ori[..16]);
        co.copy_from_slice(&ori[16..24]);
    }
}

impl Cube3 {
    pub fn replace_inverse_brute(&mut self, a: &Self) {

    }
}
