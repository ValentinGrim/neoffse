//! CBC mode over [`Rijndael256`], in place.

use super::block::Rijndael256;
use super::{BLOCK_SIZE, UnalignedLength};

fn xor_into(block: &mut [u8; BLOCK_SIZE], mask: &[u8; BLOCK_SIZE]) {
    for (byte, mask_byte) in block.iter_mut().zip(mask) {
        *byte ^= mask_byte;
    }
}

impl Rijndael256 {
    /// Encrypts `data` in place in CBC mode.
    ///
    /// Block 1 is combined by XOR with `iv`, each later block with the previous
    /// ciphertext block. Adds no padding: call
    /// [`pad_zeros`](super::pad_zeros) first when the length is not aligned.
    /// An empty slice is a valid empty message.
    ///
    /// # Errors
    ///
    /// Returns [`UnalignedLength`] when `data.len()` is not a multiple of 32.
    /// The check runs before any write, so `data` is unchanged on error.
    ///
    /// # Examples
    ///
    /// ```
    /// use neoffse_format_pr::cipher::{PR_IV, Rijndael256, pad_zeros};
    ///
    /// let cipher = Rijndael256::pr();
    /// let mut data = b"hello".to_vec();
    /// pad_zeros(&mut data);
    /// cipher.cbc_encrypt(&PR_IV, &mut data)?;
    /// cipher.cbc_decrypt(&PR_IV, &mut data)?;
    /// assert_eq!(data.len(), 32);
    /// assert!(data.starts_with(b"hello"));
    /// # Ok::<(), neoffse_format_pr::cipher::UnalignedLength>(())
    /// ```
    pub fn cbc_encrypt(&self, iv: &[u8; 32], data: &mut [u8]) -> Result<(), UnalignedLength> {
        let len = data.len();
        let (blocks, rest) = data.as_chunks_mut::<BLOCK_SIZE>();
        if !rest.is_empty() {
            return Err(UnalignedLength { len });
        }
        let mut previous = *iv;
        for block in blocks {
            xor_into(block, &previous);
            *block = self.encrypt_block(block);
            previous = *block;
        }
        Ok(())
    }

    /// Decrypts `data` in place in CBC mode and strips nothing.
    ///
    /// The output has the input length, trailing zero bytes included: the
    /// Go `Decrypt` cuts up to 31 of them and loses stream bytes (see
    /// `docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap`).
    /// An empty slice is a valid empty message.
    ///
    /// # Errors
    ///
    /// Returns [`UnalignedLength`] when `data.len()` is not a multiple of 32.
    /// The check runs before any write, so `data` is unchanged on error.
    pub fn cbc_decrypt(&self, iv: &[u8; 32], data: &mut [u8]) -> Result<(), UnalignedLength> {
        let len = data.len();
        let (blocks, rest) = data.as_chunks_mut::<BLOCK_SIZE>();
        if !rest.is_empty() {
            return Err(UnalignedLength { len });
        }
        let mut previous = *iv;
        for block in blocks {
            let ciphertext = *block;
            *block = self.decrypt_block(&ciphertext);
            xor_into(block, &previous);
            previous = ciphertext;
        }
        Ok(())
    }
}
