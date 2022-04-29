#![allow(dead_code)]

use bitpacking::{BitPacker4x, BitPacker};

const BLOCK_SIZE: usize = 128;

trait Storage {
    fn store(&mut self, value: u32);
    fn store_batch(&mut self, values: &[u32]);
}

/// Stores compressed blocks and their metadata
struct CompressedStorage {
    bitpacker: BitPacker4x,
    blocks: Vec<StorageBlock>,
    compressed: Vec<u8>,
    buffer: Vec<u32>, 
}

/// The metadata required to manage a compressed block
struct StorageBlock {
    /// The delta encoding of the initial element of the block
    initial: u32,
    /// The start index of compressed block inside the compressed storage 
    start: usize,
    /// The number of bits that each element was compressed to
    bits: u8,
}

impl CompressedStorage {
    fn new() -> Self {
        Self {
            bitpacker: BitPacker4x::new(),
            blocks: Vec::new(),
            compressed: Vec::new(),
            buffer: Vec::with_capacity(BLOCK_SIZE),
        }
    }

    fn compress_buffer(&mut self) {
        // Ensure that the bufffer is full as only complete blocks can be compressed
        debug_assert_eq!(self.buffer.len(), BLOCK_SIZE);

        // Find the initial value for delta encoding the first value
        let initial = *self.buffer.get(0).expect("buffer[0] missing");

        // Calculate the number of bits and bytes of the compressed data
        let bits = self.bitpacker.num_bits_sorted(initial, &self.buffer);
        let bytes = BitPacker4x::BLOCK_LEN * (bits as usize) / 8;

        // Compress 
        let mut block = vec![0u8; bytes];
        self.bitpacker.compress_sorted(initial, &self.buffer, &mut block, bits);

        // Write the compressed block metadata to the block metadata store
        self.blocks.push(StorageBlock {
            bits,
            initial,
            start: self.compressed.len(),
        });

        // Ensure we can recover the compressed block length from just the number of bits, given
        // the block size is fixed at 128 elements
        debug_assert_eq!(block.len(), bytes);
       
        // Write the compressed block into the compressed store
        self.compressed.extend_from_slice(&block);
        
        let bitpacker = BitPacker4x::new();
        let start = 100000;
        let original: Vec<u32> = (start..start+896).filter(|i| i % 7 == 0).collect();

        // Calculate the number of compressed bytes
        let num_bits = bitpacker.num_bits_sorted(start, &original);
        let compressed_bytes = BitPacker4x::BLOCK_LEN * (num_bits as usize) / 8;

        // Compress
        let mut compressed = vec![0u8; compressed_bytes];
        bitpacker.compress_sorted(start, &original, &mut compressed, num_bits);

        // Clear the buffer 
        self.buffer.clear();
    }

    fn decompress_block(&self, block: usize) -> Vec<u32> {
        let block = self.blocks.get(block).expect("block exists");
        let bytes = BitPacker4x::BLOCK_LEN * (block.bits as usize) / 8;
        let compressed = &self.compressed[block.start..block.start+bytes];

        let mut decompressed = vec![0u32; 128];
        self.bitpacker.decompress_sorted(block.initial, compressed, &mut decompressed, block.bits);

        decompressed
    }

    fn add(&mut self, value: u32) {
        self.buffer.push(value);
        if self.buffer.len() == BLOCK_SIZE {
            self.compress_buffer();             
        }
    }

    fn add_batch(&mut self, values: &[u32]) {
        for value in values {
            self.add(*value);
        }
    }

    /// Finds the block which must contain the the search value. 
    /// As the blocks are totally ordered, this will always be the prior block to the first block
    /// with a greater initial value than the search value.
    /// The block is found using modified binary search in O(logN) time.
    /// TODO: Write tests for edge cases
    fn find_block(&self, value: u32) -> Option<usize> {
        let mut left = 0;
        let mut right = self.blocks.len();

        while left < right {
            let mid = (left + right) / 2;
            if self.blocks.get(mid).expect("mid block exists").initial < value + 1 {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        let block = left-1;

        // Check that we haven't missed the search value by error
        let next = self.blocks.get(block+1);
        debug_assert!(next.is_none() || next.unwrap().initial > value);

        Some(block)
    }

    /// Finds the value within the block.
    /// This is achieved by a sequential scan but could be done using a binary search if that
    /// proves to be faster for the 128-element blocks.
    fn find_value(&self, value: u32, block: usize) -> Option<usize> {
        let decompressed = self.decompress_block(block);
        for (i, v) in decompressed.iter().enumerate() {
            if v == &value {
                return Some(block * BLOCK_SIZE + i)
            }
        }
        None
    }

    /// Checks if the value exists inside the compressed storage
    fn find(&self, value: u32) -> Option<usize> {
        match self.find_block(value) {
            None => None,
            Some(block) => self.find_value(value, block)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage() {
        let mut storage = CompressedStorage::new();

        (100..1000)
            .filter(|i| i % 2 == 0)
            .for_each(|i| storage.add(i));

        assert_eq!(storage.find(560), Some(230));
        assert_eq!(storage.find(561), None);
    }
    
    #[test]
    fn test() {
        let bitpacker = BitPacker4x::new();
        let start = 100000;
        let original: Vec<u32> = (start..start+896).filter(|i| i % 7 == 0).collect();

        // Calculate the number of compressed bytes
        let num_bits = bitpacker.num_bits_sorted(start, &original);
        let compressed_bytes = BitPacker4x::BLOCK_LEN * (num_bits as usize) / 8;

        // Compress
        let mut compressed = vec![0u8; compressed_bytes];
        bitpacker.compress_sorted(start, &original, &mut compressed, num_bits);

        // Decompress
        let mut decompressed: Vec<u32> = vec![0u32; 128];
        bitpacker.decompress_sorted(start, &compressed, &mut decompressed, num_bits);

        println!("original {}-bits", 32 * original.len());
        println!("compressed {}-nbits {}-bits", num_bits, 8 * compressed.len());
        println!("decompressed {}-bits", 32 * decompressed.len());
        assert_eq!(original, decompressed);
    }
}
