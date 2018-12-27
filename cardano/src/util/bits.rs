const NUM_BITS_PER_BLOCK: usize = 11;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum State {
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
}
impl State {
    fn index(&self) -> usize {
        match self {
            State::S0 => 0,
            State::S1 => 3,
            State::S2 => 6,
            State::S3 => 1,
            State::S4 => 4,
            State::S5 => 7,
            State::S6 => 2,
            State::S7 => 5,
        }
    }
}

pub struct BitWriterBy11 {
    buffer: Vec<u8>,
    state: State,
}

impl BitWriterBy11 {
    pub fn new() -> Self {
        BitWriterBy11 {
            buffer: Vec::new(),
            state: State::S0,
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        self.buffer
    }

    pub fn write(&mut self, e: u16) {
        match self.state {
            State::S0 => {
                let x = e >> 3 & 0b1111_1111;
                let y = e << 5 & 0b1110_0000;
                self.buffer.push(x as u8);
                self.buffer.push(y as u8);
                self.state = State::S1;
            }
            State::S1 => {
                let x = e >> 6 & 0b0001_1111;
                let y = e << 2 & 0b1111_1100;
                if let Some(last) = self.buffer.last_mut() {
                    *last |= x as u8;
                } else {
                    unreachable!()
                }
                self.buffer.push(y as u8);
                self.state = State::S2;
            }
            State::S2 => {
                let x = e >> 9 & 0b0000_0011;
                let y = e >> 1 & 0b1111_1111;
                let z = e << 7 & 0b1000_0000;
                if let Some(last) = self.buffer.last_mut() {
                    *last |= x as u8;
                } else {
                    unreachable!()
                }
                self.buffer.push(y as u8);
                self.buffer.push(z as u8);
                self.state = State::S3;
            }
            State::S3 => {
                let x = e >> 4 & 0b0111_1111;
                let y = e << 4 & 0b1111_0000;
                if let Some(last) = self.buffer.last_mut() {
                    *last |= x as u8;
                } else {
                    unreachable!()
                }
                self.buffer.push(y as u8);
                self.state = State::S4;
            }
            State::S4 => {
                let x = e >> 7 & 0b0000_1111;
                let y = e << 1 & 0b1111_1110;
                if let Some(last) = self.buffer.last_mut() {
                    *last |= x as u8;
                } else {
                    unreachable!()
                }
                self.buffer.push(y as u8);
                self.state = State::S5;
            }
            State::S5 => {
                let x = e >> 10 & 0b0000_0001;
                let y = e >> 2 & 0b1111_1111;
                let z = e << 6 & 0b1100_0000;
                if let Some(last) = self.buffer.last_mut() {
                    *last |= x as u8;
                } else {
                    unreachable!()
                }
                self.buffer.push(y as u8);
                self.buffer.push(z as u8);
                self.state = State::S6;
            }
            State::S6 => {
                let x = e >> 5 & 0b0011_1111;
                let y = e << 3 & 0b1111_1000;
                if let Some(last) = self.buffer.last_mut() {
                    *last |= x as u8;
                } else {
                    unreachable!()
                }
                self.buffer.push(y as u8);
                self.state = State::S7;
            }
            State::S7 => {
                let x = e >> 8 & 0b0000_0111;
                let y = e & 0b1111_1111;
                if let Some(last) = self.buffer.last_mut() {
                    *last |= x as u8;
                } else {
                    unreachable!()
                }
                self.buffer.push(y as u8);
                self.state = State::S0;
            }
        }
    }
}

pub struct BitReaderBy11<'a> {
    buffer: &'a [u8],
    state: State,
}

impl<'a> BitReaderBy11<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        BitReaderBy11 {
            buffer: bytes,
            state: State::S0,
        }
    }

    pub fn size(&self) -> usize {
        ((self.buffer.len() * 8) - self.state.index()) / NUM_BITS_PER_BLOCK
    }

    pub fn read(&mut self) -> u16 {
        match self.state {
            State::S0 => {
                assert!(self.buffer.len() > 1);
                let x = self.buffer[0] as u16 & 0b1111_1111;
                let y = self.buffer[1] as u16 & 0b1110_0000;
                self.state = State::S1;
                self.buffer = &self.buffer[1..];
                (x << 3) | (y >> 5)
            }
            State::S1 => {
                assert!(self.buffer.len() > 1);
                let x = self.buffer[0] as u16 & 0b0001_1111;
                let y = self.buffer[1] as u16 & 0b1111_1100;
                self.state = State::S2;
                self.buffer = &self.buffer[1..];
                (x << 6) | (y >> 2)
            }
            State::S2 => {
                assert!(self.buffer.len() > 2);
                let x = self.buffer[0] as u16 & 0b0000_0011;
                let y = self.buffer[1] as u16 & 0b1111_1111;
                let z = self.buffer[2] as u16 & 0b1000_0000;
                self.state = State::S3;
                self.buffer = &self.buffer[2..];
                (x << 9) | (y << 1) | (z >> 7)
            }
            State::S3 => {
                assert!(self.buffer.len() > 1);
                let x = self.buffer[0] as u16 & 0b0111_1111;
                let y = self.buffer[1] as u16 & 0b1111_0000;
                self.state = State::S4;
                self.buffer = &self.buffer[1..];
                (x << 4) | (y >> 4)
            }
            State::S4 => {
                assert!(self.buffer.len() > 1);
                let x = self.buffer[0] as u16 & 0b0000_1111;
                let y = self.buffer[1] as u16 & 0b1111_1110;
                self.state = State::S5;
                self.buffer = &self.buffer[1..];
                (x << 7) | (y >> 1)
            }
            State::S5 => {
                assert!(self.buffer.len() > 2);
                let x = self.buffer[0] as u16 & 0b0000_0001;
                let y = self.buffer[1] as u16 & 0b1111_1111;
                let z = self.buffer[2] as u16 & 0b1100_0000;
                self.state = State::S6;
                self.buffer = &self.buffer[2..];
                (x << 10) | (y << 2) | (z >> 6)
            }
            State::S6 => {
                assert!(self.buffer.len() > 1);
                let x = self.buffer[0] as u16 & 0b0011_1111;
                let y = self.buffer[1] as u16 & 0b1111_1000;
                self.state = State::S7;
                self.buffer = &self.buffer[1..];
                (x << 5) | (y >> 3)
            }
            State::S7 => {
                assert!(self.buffer.len() > 1);
                let x = self.buffer[0] as u16 & 0b0000_0111;
                let y = self.buffer[1] as u16 & 0b1111_1111;
                self.state = State::S0;
                self.buffer = &self.buffer[2..];
                (x << 8) | y
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bit_read_by_11() {
        const BYTES: &'static [u8] = &[
            0b0000_0000,
            0b0010_0000,
            0b0000_0100,
            0b0000_0000,
            0b1000_0000,
            0b0001_0000,
            0b0000_0010,
            0b0000_0000,
            0b0100_0000,
            0b0000_1000,
            0b0000_0001,
            0b1000_0000,
            0b0011_0000,
            0b0000_0110,
            0b0000_0000,
            0b1100_0000,
            0b0001_1000,
            0b0000_0011,
            0b0000_0000,
            0b0110_0000,
            0b0000_1100,
            0b0000_0001,
        ];
        let mut reader = BitReaderBy11::new(BYTES);

        let byte = reader.read();
        assert_eq!(byte, 0b000_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b000_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b000_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b000_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b000_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b000_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b000_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b000_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b100_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b100_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b100_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b100_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b100_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b100_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b100_0000_0001);
        let byte = reader.read();
        assert_eq!(byte, 0b100_0000_0001);
    }
}
