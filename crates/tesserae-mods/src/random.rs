//! The Mod's WASI random sources, a pure function of the World seed and the Mod id.

use std::convert::Infallible;

use rand_core::TryRng;
use sha2::{Digest, Sha256};

/// SplitMix64: small and good enough for a sandboxed guest; secrecy is not the goal, repeatability is.
pub(crate) struct SeededRng(u64);

impl SeededRng {
    /// The secure source, the insecure source and the insecure seed, each a different stream.
    pub(crate) fn for_mod(world_seed: u64, id: &str) -> (SeededRng, SeededRng, u128) {
        let mut hasher = Sha256::new();
        hasher.update(b"tesserae-mod-random");
        hasher.update(world_seed.to_le_bytes());
        hasher.update(id.as_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        let word = |i: usize| u64::from_le_bytes(digest[i * 8..i * 8 + 8].try_into().unwrap());
        (
            SeededRng(word(0)),
            SeededRng(word(1)),
            u128::from(word(2)) << 64 | u128::from(word(3)),
        )
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl TryRng for SeededRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok((self.next() >> 32) as u32)
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        Ok(self.next())
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Infallible> {
        for chunk in dest.chunks_mut(8) {
            chunk.copy_from_slice(&self.next().to_le_bytes()[..chunk.len()]);
        }
        Ok(())
    }
}
