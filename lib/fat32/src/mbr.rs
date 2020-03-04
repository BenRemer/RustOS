use core::fmt;
use shim::const_assert_size;
use shim::io;
use core::mem;

use crate::traits::BlockDevice;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    // Fill me in.
    head: u8,
    sector: u8,
    cylinder: u8,
}

// implement Debug for CHS
impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CHS").finish()
    }
}
const_assert_size!(CHS, 3);

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)] // implement Debug for PartitionEntry
pub struct PartitionEntry {
    // Fill me in.
    boot_indicator: u8,
    first_sector: CHS,
    pub partition_type: u8,
    last_sector: CHS,
    pub first_sector_lba: u32,
    pub num_sectors: u32,
}

const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    // Fill me in.
    bootstrap: [u8; 436],
    id: [u8; 10],
    pub partitions: [PartitionEntry; 4],
    signature: u16,
}

// implement Debug for MaterBootRecord
impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MasterBootRecord")
            .field("partitions", &self.partitions)
            .finish()
    }
}

const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partition `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl PartitionEntry {
    const OPTION1: u8 = 0x0B;
    const OPTION2: u8 = 0x0C;

    pub fn is_fat(&self) -> bool {
        let p_type = self.partition_type;
        return p_type == PartitionEntry::OPTION1 || p_type == PartitionEntry::OPTION2
    }
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut buf = [0u8; 512];
        device.read_sector(0, &mut buf).map_err(|err| {Error::Io(err)})?;
        let mbr: MasterBootRecord = unsafe {mem::transmute(buf)};
        if mbr.signature != 0xAA55 {
            return Err(Error::BadSignature);
        }
        for i in 0..4 {
            if mbr.partitions[i].boot_indicator != 0x00 && mbr.partitions[i].boot_indicator != 0x80 {
                return Err(Error::UnknownBootIndicator(i as u8));
            }
        }
        Ok(mbr)
    }

    pub fn get_partition(&self, partition: usize) -> PartitionEntry {
        self.partitions[partition]
    }
}
