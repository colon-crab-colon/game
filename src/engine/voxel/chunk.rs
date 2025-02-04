use ahash::HashMap;

use super::quad::Quad;

pub const CHUNK_SIZE: usize = 32;
pub const VOXEL_SIZE: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    Front = 4,
    Back = 5,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    X,
    Y,
    Z,
}

// The chunk coordinates are left handed
pub struct Chunk {
    voxels: Box<[u32; 32 * 32]>,
    colors: HashMap<u16, u8>,
}

impl Chunk {
    pub fn empty() -> Chunk {
        Chunk {
            voxels: Box::new([0u32; 32 * 32]),
            colors: HashMap::default(),
        }
    }

    /// Sets voxel state inside a chunk
    /// The voxel coordinate system is left handed
    pub fn set(&mut self, x: usize, y: usize, z: usize, state: bool, color: u8) {
        assert!(x < CHUNK_SIZE);
        assert!(y < CHUNK_SIZE);
        assert!(z < CHUNK_SIZE);

        self.colors
            .insert(((z * 32 * 32) + ((31 - y) * 32) + x) as u16, color);

        if state {
            self.voxels[(z * 32) + (31 - y)] |= 2147483648 >> x;
        } else {
            self.voxels[(z * 32) + (31 - y)] &= u32::MAX ^ (2147483648 >> x);
        }
    }

    /// Gets a voxel state inside a chunk
    /// The voxel coordinate system is left handed
    pub fn get_occupied(&self, x: usize, y: usize, z: usize) -> bool {
        assert!(x < CHUNK_SIZE);
        assert!(y < CHUNK_SIZE);
        assert!(z < CHUNK_SIZE);

        self.voxels[(z * 32) + (31 - y)] & (2147483648 >> x) != 0
    }

    pub fn get_color(&self, x: usize, y: usize, z: usize) -> Option<u8> {
        assert!(x < CHUNK_SIZE);
        assert!(y < CHUNK_SIZE);
        assert!(z < CHUNK_SIZE);

        self.colors
            .get(&(((z * 32 * 32) + ((31 - y) * 32) + x) as u16))
            .copied()
    }

    pub fn remesh(&mut self, out: &mut Vec<Quad>) {
        let mut buffer = [[0u32; 32]; 34];

        // X-Axis
        for n in 0..32 {
            self.slice(Axis::X, n, &mut buffer[n + 1]);
        }

        for n in 1..33 {
            // "Vertical"
            for a in 0..32 {
                // "Horizontal"
                for b in 0..32 {
                    let right = &buffer[n - 1];
                    let mid = &buffer[n];
                    let left = &buffer[n + 1];

                    if left[a] & (2147483648 >> b) == 0 && mid[a] & (2147483648 >> b) != 0 {
                        out.push(Quad::new(
                            Direction::Left,
                            n - 1,
                            31 - a,
                            b,
                            self.get_color(n - 1, 31 - a, b).unwrap() as usize,
                        ));
                    }
                    if mid[a] & (2147483648 >> b) != 0 && right[a] & (2147483648 >> b) == 0 {
                        out.push(Quad::new(
                            Direction::Right,
                            n - 1,
                            31 - a,
                            b,
                            self.get_color(n - 1, 31 - a, b).unwrap() as usize,
                        ));
                    }
                }
            }
        }

        // Y-Axis
        for n in 0..32 {
            self.slice(Axis::Y, n, &mut buffer[n + 1]);
        }

        for n in 1..33 {
            // "Vertical"
            for a in 0..32 {
                // "Horizontal"
                for b in 0..32 {
                    let up = &buffer[n + 1];
                    let mid = &buffer[n];
                    let down = &buffer[n - 1];

                    if up[a] & (2147483648 >> b) == 0 && mid[a] & (2147483648 >> b) != 0 {
                        out.push(Quad::new(
                            Direction::Up,
                            b,
                            n - 1,
                            31 - a,
                            self.get_color(b, n - 1, 31 - a).unwrap() as usize,
                        ));
                    }
                    if mid[a] & (2147483648 >> b) != 0 && down[a] & (2147483648 >> b) == 0 {
                        out.push(Quad::new(
                            Direction::Down,
                            b,
                            n - 1,
                            31 - a,
                            self.get_color(b, n - 1, 31 - a).unwrap() as usize,
                        ));
                    }
                }
            }
        }

        // Z-Axis
        for n in 0..32 {
            self.slice(Axis::Z, n, &mut buffer[n + 1]);
        }

        for n in 1..33 {
            // "Vertical"
            for a in 0..32 {
                // "Horizontal"
                for b in 0..32 {
                    let front = &buffer[n - 1];
                    let mid = &buffer[n];
                    let back = &buffer[n + 1];

                    if front[a] & (2147483648 >> b) == 0 && mid[a] & (2147483648 >> b) != 0 {
                        out.push(Quad::new(
                            Direction::Front,
                            b,
                            31 - a,
                            n - 1,
                            self.get_color(b, 31 - a, n - 1).unwrap() as usize,
                        ));
                    }
                    if mid[a] & (2147483648 >> b) != 0 && back[a] & (2147483648 >> b) == 0 {
                        out.push(Quad::new(
                            Direction::Back,
                            b,
                            31 - a,
                            n - 1,
                            self.get_color(b, 31 - a, n - 1).unwrap() as usize,
                        ));
                    }
                }
            }
        }

        /*
        for quad in out.iter_mut() {
            match quad.direction() {
                Direction::Down => quad.set_texture_id(0),
                Direction::Up => quad.set_texture_id(24),
                Direction::Left => quad.set_texture_id(48),
                Direction::Right => quad.set_texture_id(72),
                Direction::Front => quad.set_texture_id(96),
                Direction::Back => quad.set_texture_id(120),
            }
        }
        */
    }

    fn slice(&self, axis: Axis, n: usize, buffer: &mut [u32; 32]) {
        match axis {
            Axis::X =>
            {
                #[allow(clippy::needless_range_loop)]
                for y in 0..32 {
                    for z in 0..32 {
                        buffer[y] |= ((self.voxels[z * 32 + y] << n) & 2147483648) >> z;
                    }
                }
            }
            Axis::Y => {
                for z in 0..32 {
                    buffer[31 - z] = self.voxels[z * 32 + (31 - n)]
                }
            }
            Axis::Z =>
            {
                #[allow(clippy::needless_range_loop)]
                for y in 0..32 {
                    buffer[y] = self.voxels[(n * 32) + y];
                }
            }
        }
    }

    pub fn count(&self) -> usize {
        let mut count = 0;

        for n in self.voxels.as_slice() {
            count += n.count_ones() as usize;
        }

        count
    }
}

#[test]
fn test_set_get() {
    for z in 0..32 {
        for y in 0..32 {
            for x in 0..32 {
                let mut chunk = Chunk::empty();

                chunk.set(x, y, z, true, 0);
                assert!(chunk.get_occupied(x, y, z));
            }
        }
    }
}

#[test]
fn test_slice() {
    let mut target = [u32::MAX; 32];

    target[30] ^= 1 << 30;

    let mut buffer = [0u32; 32];

    // X-Axis
    for n in 0..32 {
        let mut chunk = Chunk::empty();

        for y in 0..32 {
            for z in 0..32 {
                chunk.set(n, z, y, true, 0);
            }
        }

        chunk.set(n, 1, 1, false, 0);

        chunk.slice(Axis::X, n, &mut buffer);

        assert_eq!(buffer, target);
    }

    // Y-Axis
    for n in 0..32 {
        let mut chunk = Chunk::empty();

        for x in 0..32 {
            for z in 0..32 {
                chunk.set(x, n, z, true, 0);
            }
        }

        chunk.set(1, n, 1, false, 0);

        chunk.slice(Axis::Y, n, &mut buffer);

        assert_eq!(buffer, target);
    }

    // Z-Axis
    for n in 0..32 {
        let mut chunk = Chunk::empty();

        for y in 0..32 {
            for x in 0..32 {
                chunk.set(x, y, n, true, 0);
            }
        }

        chunk.set(1, 1, n, false, 0);

        chunk.slice(Axis::Z, n, &mut buffer);

        assert_eq!(buffer, target);
    }
}
