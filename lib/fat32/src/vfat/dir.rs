use alloc::string::String;
use alloc::vec::Vec;
use crate::alloc::string::ToString;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;
use shim::ioerr;

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle};

use core::str::{from_utf8};
use core::mem::size_of;

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE, // File system Handle to Dir
    // Fill me in.
    pub first_cluster: Cluster,
    // pub name: String,
    pub name: alloc::string::String,
    // pub name_long: String,
    pub metadata: Metadata,
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    pub fn metadata(&self) -> &Metadata {
        return &self.metadata;
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    // Fill me in.
    pub file_name: [u8; 8],
    pub file_extension: [u8; 3],
    pub attributes: Attributes,
    pub windows_nt_flag: u8,
    pub created_time: u8,
    pub creation_time: Time,
    pub creation_date: Date,
    pub accessed_date: Date,
    pub high_two_bytes: u16, // of first cluster
    // pub metadata: Metadata,
    pub modified_time: Time,
    pub modified_date: Date,
    pub low_two_bytes: u16, // of first cluster
    pub file_size: u32,
}

const_assert_size!(VFatRegularDirEntry, 32);

impl VFatRegularDirEntry {
    pub fn first_cluster(&self) -> u32 {
        return ((self.high_two_bytes as u32) << 16) | self.low_two_bytes as u32;
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    pub sequence_num: u8,
    pub name_extra: [u16; 5],
    pub attributes: Attributes,
    pub type_lfn: u8, 
    pub checksum: u8,
    pub name_extra_2: [u16; 6],
    pub signature: u16, 
    pub name_extra_3: [u16; 2],
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    pub status: u8,
    __r1: [u8; 10],
    pub attributes: Attributes,
    __r2: [u8; 20],
}

const_assert_size!(VFatUnknownDirEntry, 32);

#[repr(C, packed)]
#[derive(Default, Copy, Clone)]
pub struct VFatDummyDirEntry {
    __r1: [u8; 32], 
}

#[derive(Copy, Clone)]
pub union VFatDirEntry {
    pub unknown: VFatUnknownDirEntry,
    pub regular: VFatRegularDirEntry,
    pub long_filename: VFatLfnDirEntry,
    // dummy: VFatDummyDirEntry,
}

pub struct DirIterator<HANDLE> {
    pub vfat: HANDLE,
    pub entries: Vec<VFatDirEntry>,
    pub curr_position: usize,
    pub bytes_per_cluster: u32,
}

impl<HANDLE: VFatHandle> Iterator for DirIterator<HANDLE> {
    type Item = Entry<HANDLE>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = [0u16; 260];
        let mut has_lfn = false;
        while self.curr_position < self.entries.len() {
            let curr_entry = &self.entries[self.curr_position];
            let as_unknown = unsafe{curr_entry.unknown};

            match as_unknown.status {
                0x00 => return None,
                0xE5 => {
                    self.curr_position += 1; 
                    continue;
                },
                _ => ()
            };

            self.curr_position += 1;

            if as_unknown.attributes.0 == 0x0F { // is lfn
                let lfn_entry = unsafe {curr_entry.long_filename};
                let mut name_chars = [0u16; 13];
                name_chars[..5].clone_from_slice(&{lfn_entry.name_extra});
                name_chars[5..11].clone_from_slice(&{lfn_entry.name_extra_2});
                name_chars[11..13].clone_from_slice(&{lfn_entry.name_extra_3});
                let mut end_name_chars = 0;
                for i in 0..name_chars.len() {
                    end_name_chars = i;
                    if name_chars[i] == 0x00 {
                        break;
                    }
                }
                let start_index = (((lfn_entry.sequence_num & 0x1F) -1) as usize) * 13;
                buf[start_index..=start_index + end_name_chars]
                    .clone_from_slice(&name_chars[..=end_name_chars]);
                has_lfn = true;
                continue;
            } else {
                let regular_entry = unsafe{curr_entry.regular};
                // print!("{}", regular_entry.attributes.0);
                let name = match has_lfn {
                    true => {
                        let mut last = 0;
                        for i in 0..buf.len() {
                            if buf[i] != 0 {
                                last += 1;
                            } else {
                                break;
                            }
                        }
                        String::from(String::from_utf16(&buf[..last]).unwrap().trim_end())
                    },
                    false => {
                        let filename = core::str::from_utf8(&regular_entry.file_name).unwrap().trim_end();
                        let extension = core::str::from_utf8(&regular_entry.file_extension).unwrap().trim_end();
                        let final_name;
                        if extension.len() > 0 {
                            final_name = format!("{}.{}", filename, extension)
                        } else {
                            final_name = String::from(filename).trim_end().to_string();
                        }
                        final_name
                    }
                };
                let metadata = Metadata {
                    attributes: regular_entry.attributes,
                    created: Timestamp{date: regular_entry.creation_date, time: regular_entry.creation_time},
                    accessed: Timestamp{date:regular_entry.accessed_date, time: Time::default()},
                    modified: Timestamp{date: regular_entry.modified_date, time: regular_entry.modified_time},
                    start_cluster: Cluster((regular_entry.high_two_bytes as u32) << 16 | 
                        regular_entry.low_two_bytes as u32)
                };

                if (regular_entry.attributes.0 & 0x10) != 0 {
                    return Some(
                        Entry::DirEntry(Dir {
                            vfat: self.vfat.clone(),
                            first_cluster: (&metadata).start_cluster,
                            metadata: metadata,
                            name: name
                        })
                    );
                } else {
                    return Some(
                        Entry::FileEntry(File {
                            vfat: self.vfat.clone(),
                            first_cluster: (&metadata).start_cluster.clone(),
                            curr_cluster: Some((&metadata).start_cluster.clone()),
                            name: name,
                            metadata: metadata,
                            curr_offset: 0,
                            size: regular_entry.file_size as u64,
                            cluster_size: self.bytes_per_cluster as u64
                        })
                    );
                }
            }
        }
        None
    }
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        use traits::Dir;
        for entry in self.entries()? {
            let upper = match &entry {
                Entry::FileEntry(file) => &file.name,
                Entry::DirEntry(dir) => &dir.name,
            };
            let lower = match name.as_ref().to_str() {
                Some(s) => s,
                None => return ioerr!(InvalidInput, "Name invalid")
            };
            if str::eq_ignore_ascii_case(upper, lower) {
                return Ok(entry);
            }
        }
        return ioerr!(NotFound, "not found");
    }

    pub fn new(vfat: HANDLE) -> Dir<HANDLE> {
        let cluster = vfat.lock(|vfat| vfat.get_root_cluster());
        return Dir {
            vfat: vfat.clone(),
            first_cluster: cluster,
            name: String::new(),
            metadata: Metadata::default(),
        };
    }
}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    type Entry = Entry<HANDLE>;

    type Iter = DirIterator<HANDLE>;

    /// Returns an interator over the entries in this directory.
    fn entries(&self) -> io::Result<Self::Iter> {
        let mut vec: Vec<u8> = Vec::new();
        let bytes_per_cluster = {
            self.vfat.lock(|vfat| vfat.read_chain(self.first_cluster, &mut vec).expect("Read without error"));
            let bpc = self.vfat.lock(|vfat| 
                vfat.get_bytes_per_sector() * vfat.sectors_per_cluster as usize) as u32;
            bpc
        };
        let iterator = DirIterator {
            vfat: self.vfat.clone(),
            entries: unsafe {vec.cast()},
            curr_position: 0,
            bytes_per_cluster: bytes_per_cluster,
        };
        Ok(iterator)
    }
}