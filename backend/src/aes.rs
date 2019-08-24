use aes_ctr::stream_cipher::generic_array::*;
use aes_ctr::stream_cipher::*;
use aes_ctr::*;
use crate::key;

pub fn aes_ctr_decrypt(data: &mut [u8], key: &[u8; 16], ctr: &[u8; 16], offset: u64) {
    let key = GenericArray::from_slice(key);
    let ctr = GenericArray::from_slice(ctr);
    let mut cipher = Aes128Ctr::new(&key, &ctr);
    cipher.seek(offset);
    cipher.apply_keystream(data);
}

fn lrot128(a: &[u8], rot: usize) -> [u8; 16] {
    let mut out = [0; 16];
    let byte_shift = rot / 8;
    let bit_shift = rot % 8;
    for (i, o) in out.iter_mut().enumerate() {
        let wrap_index_a = (i + byte_shift) % 16;
        let wrap_index_b = (i + byte_shift + 1) % 16;
        // note: the right shift would be UB for bit_shift = 0.
        // good thing is that the values we will use for rot won't cause this
        *o = (a[wrap_index_a] << bit_shift) | (a[wrap_index_b] >> (8 - bit_shift));
    }
    out
}

fn add128(a: &[u8], b: &[u8]) -> [u8; 16] {
    let mut out = [0; 16];
    let mut carry = 0;

    for i in (0..16).rev() {
        let sum = u32::from(a[i]) + u32::from(b[i]) + carry;
        carry = sum >> 8;
        out[i] = (sum & 0xFF) as u8;
    }
    out
}

fn xor128(a: &[u8], b: &[u8]) -> [u8; 16] {
    let mut out = [0; 16];
    for i in 0..16 {
        out[i] = a[i] ^ b[i];
    }
    out
}

fn scramble(x: &[u8], y: &[u8]) -> [u8; 16] {
    lrot128(&add128(&xor128(&lrot128(x, 2), y), &*key::SCRAMBLER), 87)
}

pub fn get_ncch_key(y: &[u8]) -> [u8; 16] {
    scramble(&*key::KEY_X, y)
}
