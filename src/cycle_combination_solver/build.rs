use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        simd8: {
            all(
                any(
                    target_arch = "aarch64",
                    target_arch = "arm64ec",
                    all(
                        target_arch = "arm",
                        target_feature = "v7"
                    )
                ),
                target_feature = "neon",
                target_endian = "little"
            )
        },
        simd16: {
            any(
                target_feature = "ssse3",
                target_feature = "simd128",
                all(
                    any(
                        target_arch = "aarch64",
                        target_arch = "arm64ec"
                    ),
                    target_feature = "neon",
                    target_endian = "little"
                )
            )
        },
        simd8and16: {
            all(
                simd8,
                simd16
            )
        },
        // simd8and16: { not(l) }, // true
        // simd8and16: { l }, // false
        simd32: {
            any(
                all(
                    target_feature = "avx2",
                    not(target_feature = "avx512vbmi")
                ),
                all(
                    target_feature = "avx512vl",
                    target_feature = "avx512vbmi"
                )
            )
        },
        // simd32: { not(l) }, // true
        // simd32: { l }, // false
    }
}
