//! Private, fixed SHA-256 encoder for the admitted complete policy identity.
//! No dependency or public hashing API is added outside the admitted Core
//! manifest. This implements only SHA-256, with fixed stack storage; policy
//! encodings are checked against the independently registered canonical vectors.
use super::*;

struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    len: usize,
    bytes: u64,
}
impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            block: [0; 64],
            len: 0,
            bytes: 0,
        }
    }
    fn update(&mut self, input: &[u8]) {
        self.bytes += input.len() as u64;
        for &b in input {
            self.block[self.len] = b;
            self.len += 1;
            if self.len == 64 {
                self.compress();
                self.len = 0;
            }
        }
    }
    fn compress(&mut self) {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut w = [0u32; 64];
        for (out, input) in w[..16].iter_mut().zip(self.block.chunks_exact(4)) {
            *out = u32::from_be_bytes(input.try_into().unwrap());
        }
        for i in 16..64 {
            let x = w[i - 15];
            let y = w[i - 2];
            let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add((e & f) ^ (!e & g))
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let t2 = s0.wrapping_add((a & b) ^ (a & c) ^ (b & c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (state, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(value);
        }
    }
    fn finish(mut self) -> [u8; 32] {
        let bits = self.bytes * 8;
        self.update(&[0x80]);
        while self.len != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        let mut out = [0; 32];
        for (bytes, value) in out.chunks_exact_mut(4).zip(self.state) {
            bytes.copy_from_slice(&value.to_be_bytes());
        }
        out
    }
    fn coordinate(&mut self, bytes: &[u8]) {
        self.update(&(bytes.len() as u16).to_le_bytes());
        self.update(bytes);
    }
}

pub(super) fn policy_key(p: &ConsumerPropertyReadPolicy) -> BindingConfigurationDigest {
    let mut values = Sha256::new();
    values.update(b"clinkz-resource-values-v1\0");
    for value in p.limits.as_values() {
        match value {
            None => values.update(&[0]),
            Some(n) => {
                values.update(&[1]);
                values.update(&n.to_le_bytes());
            }
        }
    }
    let mut h = Sha256::new();
    h.update(b"clinkz-consumer-policy-identity-v1\0");
    h.coordinate(if p.contract == BindingResourceContract::StrictV1 {
        b"1"
    } else {
        b"2"
    });
    for c in [
        b"2".as_slice(),
        b"consumer-property-read",
        b"consumer-only",
        b"manual-poll",
        if cfg!(feature = "std") {
            b"std"
        } else if cfg!(feature = "async") {
            b"async-no-std"
        } else {
            b"no-default"
        },
        if p.contract == BindingResourceContract::ConsumerOwnedHostV2 {
            b"host-process-failure"
        } else {
            b"strict-local"
        },
    ] {
        h.coordinate(c);
    }
    let mut decimal = [0; 5];
    let mut start = decimal.len();
    let mut n = p.origin.get();
    loop {
        start -= 1;
        decimal[start] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    h.coordinate(&decimal[start..]);
    h.update(&values.finish());
    for f in [p.registration, p.call] {
        h.update(&f.retained_items().to_le_bytes());
        h.update(&f.retained_bytes().to_le_bytes());
    }
    for a in [
        p.input.as_slice(),
        p.result.as_slice(),
        p.work.as_slice(),
        p.timeouts.as_slice(),
    ] {
        for n in a {
            h.update(&n.to_le_bytes());
        }
    }
    h.update(&p.scratch.to_le_bytes());
    BindingConfigurationDigest::new(h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sha256_padding_and_fragmentation() {
        for (input, expected) in [
            (
                b"".as_slice(),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ] {
            let mut hash = Sha256::new();
            for b in input {
                hash.update(core::slice::from_ref(b));
            }
            let expected: alloc::vec::Vec<u8> = (0..64)
                .step_by(2)
                .map(|i| u8::from_str_radix(&expected[i..i + 2], 16).unwrap())
                .collect();
            assert_eq!(hash.finish().as_slice(), expected);
        }
    }
}
