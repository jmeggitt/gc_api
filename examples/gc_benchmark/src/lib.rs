pub mod tree;

/// A small workload which runs a specified number of rounds of PRBS31. This workload only requires
/// a couple registers and does not make use of any memory. It is intended to be hard for the
/// compiler to optimize while providing a minimal workload. When seeded with a value `1u32`, it
/// takes 2,147,483,646 rounds (verified experimentally) before `1u32` is reached and the cycle
/// repeats.
///
/// See https://en.wikipedia.org/wiki/Pseudorandom_binary_sequence
///
/// As expected, inspecting the assembly produced on an x86_64 machine using rustc 1.60 shows that
/// the compiler is unable to apply any meaningful optimizations to the function.
pub fn workload(x: u32, rounds: u32) -> u32 {
    let mut value = x;
    for _ in 0..rounds {
        let new_bit = ((value >> 30) ^ (value >> 27)) & 1;
        value = ((value << 1) | new_bit) & ((1u32 << 31) - 1);
    }
    value
}
