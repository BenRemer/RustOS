use core::fmt;

use alloc::string::String;

use crate::vfat::Cluster;
use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(pub u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Copy, Clone)]
pub struct Metadata {
    // Fill me in.
    pub attributes: Attributes,
    pub created: Timestamp,
    pub accessed: Timestamp,
    pub modified: Timestamp,
    pub start_cluster: Cluster,
}

// Implement `traits::Timestamp` for `Timestamp`.
impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        ((self.date.0 >> 9) as usize & 0x7f as usize) + 1980
    }

    fn month(&self) -> u8 {
        (self.date.0 >> 5 & 0b1111) as u8
    }

    fn day(&self) -> u8 {
        (self.date.0 & 0b11111) as u8
    }

    fn hour(&self) -> u8 {
        ((self.time.0  >> 11) & 0x1F) as u8
    }

    fn minute(&self) -> u8 {
        ((self.time.0 >> 5) & 0x3F) as u8
    }

    fn second(&self) -> u8 {
        (self.time.0 & 0x1F) as u8 * 2
    }
}

impl Attributes {
    const READ_ONLY: u8 = 0x01;
    const HIDDEN: u8 = 0x02;
    const SYSTEM: u8 = 0x04;
    const VOLUME_ID: u8 = 0x08;
    const DIRECTORY: u8 = 0x10;
    const ARCHIVE: u8 = 0x20;
    const LFN: u8 = 0x0F;

    pub fn read_only(&self) -> bool {
        (self.0 & Attributes::READ_ONLY) != 0
    }

    pub fn hidden(&self) -> bool {
        (self.0 & Attributes::HIDDEN) != 0
    }

    pub fn system(&self) -> bool {
        (self.0 & Attributes::SYSTEM) != 0
    }

    pub fn volume_id(&self) -> bool {
        (self.0 & Attributes::VOLUME_ID) != 0
    }

    pub fn directory(&self) -> bool {
        (self.0 & Attributes::DIRECTORY) != 0
    }

    pub fn archive(&self) -> bool {
        (self.0 & Attributes::ARCHIVE) != 0
    }

    pub fn lfn(&self) -> bool {
        return self.0 == Attributes::LFN;
    }
}

impl Metadata {
    pub fn new(attributes: Attributes, created: Timestamp, accessed: Timestamp, modified: Timestamp, 
        start_cluster: Cluster) -> Metadata {
            Metadata {attributes, created, accessed, modified, start_cluster}
    }
    
    fn rwh(self) -> String {
        use traits::Metadata;
        let d = match self.attributes.0 & 0xF0 {
            0x10 => "d",
            _ => "-"
        };
        let w = match self.read_only() {
            true => "-",
            false => "w"
        };
        let h = match self.hidden() {
            true => "h",
            false => "-"
        };
        format!("{}r{}{}", d, w, h)
    }
}

// Implement `traits::Metadata` for `Metadata`.
impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;
    fn read_only(&self) -> bool {
        self.attributes.0 == 0x01 // read only
    }

    fn hidden(&self) -> bool {
        (self.attributes.0 & 0x02) != 0// hidden
    }

    fn created(&self) -> Self::Timestamp {
        self.created
    }

    fn accessed(&self) -> Self::Timestamp {
        self.accessed
    }

    fn modified(&self) -> Self::Timestamp {
        self.modified
    }
}

// Implement `fmt::Display` (to your liking) for `Metadata`.
impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // write!(f, "{}Z {}", self.rwh(), self.modified)
        f.debug_struct("Metadata").finish()
    }
}
