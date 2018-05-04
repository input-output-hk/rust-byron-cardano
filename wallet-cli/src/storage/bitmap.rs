use std::iter::repeat;

// Simple bitmap of constant size
pub struct Bitmap {
    data: Vec<u8>,
    size: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ByteAddr(u8);

fn addr(bit: usize) -> (usize, ByteAddr) {
    (bit / 8, ByteAddr((bit % 8) as u8))
}

impl ByteAddr {
    pub fn clear_mask(self) -> u8 {
        match self.0 {
            0 => 0b1111_1110,
            1 => 0b1111_1101,
            2 => 0b1111_1011,
            3 => 0b1111_0111,
            4 => 0b1110_1111,
            5 => 0b1101_1111,
            6 => 0b1011_1111,
            _ => 0b0111_1111,
        }
    }

    pub fn set_mask(self) -> u8 {
        match self.0 {
            0 => 0b0000_0001,
            1 => 0b0000_0010,
            2 => 0b0000_0100,
            3 => 0b0000_1000,
            4 => 0b0001_0000,
            5 => 0b0010_0000,
            6 => 0b0100_0000,
            _ => 0b1000_0000,
        }
    }

    pub fn get_shifter(self) -> u8 { self.0 }

    pub fn get_next(self) -> Option<ByteAddr> {
        if self.0 == 7 {
            None
        } else {
            Some(ByteAddr(self.0 + 1))
        }
    }
}

impl Bitmap {
    pub fn new(size: usize) -> Self {
        let v : Vec<u8> = repeat(0).take(size as usize).collect();
        Bitmap { data: v, size: size }
    }
    pub fn set_bit_to(&mut self, bit: usize, value: bool) {
        assert!(self.size > bit);
        let (byte_addr, bit_addr) = addr(bit);
        if value {
            self.data[byte_addr] |= bit_addr.set_mask();
        } else {
            self.data[byte_addr] &= bit_addr.clear_mask();
        }
    }

    pub fn get_bit(&self, bit: usize) -> bool {
        assert!(self.size > bit);
        let (byte_addr, bit_addr) = addr(bit);
        let val = (self.data[byte_addr] >> bit_addr.get_shifter()) & 0x1;
        val == 0x1
    }

    pub fn get_bits(&self, start_bit: usize, nb_bits: usize) -> Vec<bool> {
        let mut v = Vec::with_capacity(nb_bits);
        let (start_byte_addr, start_bit_addr) = addr(start_bit);

        let mut current_val = self.data[start_byte_addr] >> start_bit_addr.get_shifter();
        let mut current_byte_addr = start_byte_addr;
        let mut current_bit_addr = start_bit_addr;
        for i in 0..nb_bits {
            v[nb_bits - i - 1] = (current_val & 0x1) == 0x1;
            match current_bit_addr.get_next() {
                None           => {
                    current_byte_addr += 1;
                    current_bit_addr = ByteAddr(0);
                    current_val = self.data[current_byte_addr];
                },
                Some(new_addr) => {
                    current_bit_addr = new_addr;
                    current_val = current_val >> 1;
                },
            }
        }
        v
    }
}