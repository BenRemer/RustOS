use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use hashbrown::HashMap;
use shim::io;
// use std::io::Write;
use core::cmp::min;

use crate::traits::BlockDevice;

#[derive(Debug)]
struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
}

pub struct Partition {
    /// The physical sector where the partition begins.
    pub start: u64,
    /// Number of sectors
    pub num_sectors: u64,
    /// The size, in bytes, of a logical sector in the partition.
    pub sector_size: u64,
}

pub struct CachedPartition {
    device: Box<dyn BlockDevice>,
    cache: HashMap<u64, CacheEntry>,
    partition: Partition,
}

impl CachedPartition {
    /// Creates a new `CachedPartition` that transparently caches sectors from
    /// `device` and maps physical sectors to logical sectors inside of
    /// `partition`. All reads and writes from `CacheDevice` are performed on
    /// in-memory caches.
    ///
    /// The `partition` parameter determines the size of a logical sector and
    /// where logical sectors begin. An access to a sector `0` will be
    /// translated to physical sector `partition.start`. Virtual sectors of
    /// sector number `[0, num_sectors)` are accessible.
    ///
    /// `partition.sector_size` must be an integer multiple of
    /// `device.sector_size()`.
    ///
    /// # Panics
    ///
    /// Panics if the partition's sector size is < the device's sector size.
    pub fn new<T>(device: T, partition: Partition) -> CachedPartition
    where
        T: BlockDevice + 'static,
    {
        assert!(partition.sector_size >= device.sector_size());

        CachedPartition {
            device: Box::new(device),
            cache: HashMap::new(),
            partition: partition,
        }
    }

    /// Returns the number of physical sectors that corresponds to
    /// one logical sector.
    fn factor(&self) -> u64 {
        self.partition.sector_size / self.device.sector_size()
    }

    /// Maps a user's request for a sector `virt` to the physical sector.
    /// Returns `None` if the virtual sector number is out of range.
    fn virtual_to_physical(&self, virt: u64) -> Option<u64> {
        let new_virt = virt - self.partition.start;
        if new_virt >= self.partition.num_sectors {
            return None;
        }

        let physical_offset = new_virt * self.factor(); // - self.Partition.start
        let physical_sector = self.partition.start + physical_offset;

        Some(physical_sector)
    }

    // loads sector if not already loaded
    fn load_sector(&mut self, sector: u64) -> io::Result<()> {
        if !self.cache.contains_key(&sector) {
            // let factor = self.factor();
            let physical = self.virtual_to_physical(sector).unwrap();
            // let mut data = Vec::new();
            let mut data = Vec::with_capacity((self.device.sector_size() * self.factor())as usize);
            // let mut data = vec![0u8; (physical * self.factor()) as usize];
            for i in 0..self.factor() {
                self.device.read_all_sector(physical + i, &mut data)?;
            }
            self.cache.insert(sector, CacheEntry{
                data, 
                dirty: false
            });
        }
        Ok(())
    }

    /// Returns a mutable reference to the cached sector `sector`. If the sector
    /// is not already cached, the sector is first read from the disk.
    ///
    /// The sector is marked dirty as a result of calling this method as it is
    /// presumed that the sector will be written to. If this is not intended,
    /// use `get()` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get_mut(&mut self, sector: u64) -> io::Result<&mut [u8]> {
        self.load_sector(sector)?;
        let sec = self.cache.get_mut(&sector).unwrap();
        sec.dirty = true;
        Ok(&mut sec.data)
    }

    /// Returns a reference to the cached sector `sector`. If the sector is not
    /// already cached, the sector is first read from the disk.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get(&mut self, sector: u64) -> io::Result<&[u8]> {
        self.load_sector(sector)?;
        // let sec = self.cache.get(&sector).unwrap();
        // match self.cache.get(&sector) {
        //     Some(sec) => Ok(sec.data.as_slice()),
        //     _ => Err(io::Error::new(io::ErrorKind::Interrupted, "Vitual out of bounds get"))
        // }
        let sec = self.cache.get(&sector).unwrap();
        Ok(&sec.data)
    }
}

// Implement `BlockDevice` for `CacheDevice`. The `read_sector` and
// `write_sector` methods should only read/write from/to cached sectors.
impl BlockDevice for CachedPartition {
    fn sector_size(&self) -> u64 {
        self.partition.sector_size
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> io::Result<usize> {
        if !self.cache.contains_key(&sector) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "not cached yet"));
        }
        if buf.len() < self.sector_size() as usize {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "buf too short"));
        }
        let data = self.get(sector)?;
        let length = min(data.len(), buf.len());
        buf[..length].copy_from_slice(&data[..length]);
        Ok(length)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> io::Result<usize> {
        if !self.cache.contains_key(&sector) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "not cached yet"));
        }
        if buf.len() < self.sector_size() as usize {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "buf too short"));
        }
        let data = self.get_mut(sector)?;
        let length = min(data.len(), buf.len());
        data[..length].copy_from_slice(&buf[..length]);
        Ok(length)
    }
}

impl fmt::Debug for CachedPartition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CachedPartition")
            .field("device", &"<block device>")
            .field("cache", &self.cache)
            .finish()
    }
}
