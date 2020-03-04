use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;
use core::cmp::min;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;
use shim::path::Component;

use crate::mbr::MasterBootRecord;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::{BiosParameterBlock, CachedPartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status, Metadata};

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    device: CachedPartition,
    bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    rootdir_cluster: Cluster,
}

impl<HANDLE: VFatHandle> VFat<HANDLE> {
    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        let mbr: MasterBootRecord = MasterBootRecord::from(&mut device)?;
        let partition = match mbr.partitions.iter()
            .find(|&partition| partition.partition_type == 0xB || partition.partition_type == 0xC) {
                Some(v) => v,
                None => return Err(Error::NotFound)
            };
        let start_partition = partition.first_sector_lba as u64;
        let ebpb: BiosParameterBlock = BiosParameterBlock::from(&mut device, partition.first_sector_lba.into())?;
        let bytes_per_sector = ebpb.bytes_per_sector as u64;
        let partition: Partition = Partition {
            start: start_partition,
            num_sectors: partition.num_sectors as u64,
            sector_size: bytes_per_sector,
        };
        let cache: CachedPartition = CachedPartition::new(device, partition);
        let vfat_handle = VFat {
            phantom: PhantomData::<HANDLE>,
            device: cache,
            bytes_per_sector: ebpb.bytes_per_sector,
            sectors_per_cluster: ebpb.sectors_per_cluster,
            sectors_per_fat: ebpb.fat_sectors_2 as u32,
            fat_start_sector: start_partition + ebpb.reserved_sectors as u64,
            data_start_sector: start_partition + ebpb.data_start_sector(), 
            rootdir_cluster: Cluster::from(ebpb.root_cluster),
        };
        return Ok(HANDLE::new(vfat_handle));
    }

    // The following methods may be useful here:
    //
    //  * A method to read from an offset of a cluster into a buffer.
    //
    pub fn read_cluster( &mut self, cluster: Cluster, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        let sector_size = self.device.sector_size() as usize;
        let length = sector_size * self.sectors_per_cluster as usize;
        let mut sector = self.data_start_sector as usize 
            + cluster.cluster_offset() as usize * self.sectors_per_cluster as usize 
            + offset as usize / self.bytes_per_sector as usize;
        let size = min(buf.len(), length.saturating_sub(offset));
        let mut remaining = offset % self.bytes_per_sector as usize;
        let mut read = 0;
        loop {
            if read >= size {
                break;
            }
            let sector_data: &[u8] = self.device.get(sector as u64)?;
            let len_copy = if (size - read) < (sector_size - remaining) {
                size - read
            } else {
                sector_size - remaining
            };
            buf[read..(read + len_copy)].copy_from_slice(&sector_data[remaining..(remaining + len_copy)]);
            remaining = 0;
            sector += 1;
            read += len_copy;
        }
        return Ok(read);
    }

    //
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    //
    pub fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        // let mut read_byte = 0;
        // let mut cluster_node = start;
        // loop {
        //     buf.resize(buf.len() + self.bytes_per_sector as usize * self.sectors_per_cluster as usize, 0);
        //     read_byte += self.read_cluster(cluster_node, 0, &mut buf[read_byte..])?;
        //     let fat_entry = self.fat_entry(cluster_node)?;
        //     match fat_entry.status() {
        //         Status::Eoc(_) => {
        //             return Ok(read_byte);
        //         },
        //         Status::Data(next_cluster) => {
        //             cluster_node = next_cluster;
        //         },
        //         _ => {
        //             return Err(io::Error::new(io::ErrorKind::Other, "Read into invalid cluster"));
        //         }
        //     }
        // }
        // let mut read = 0;
        // let mut current_cluster = start;
        // loop {
        //     let entry = self.fat_entry(current_cluster)?;
        //     match entry.status() {
        //         Status::Data(next) => {
        //             read += self.get_cluster_data(current_cluster, buf)?;
        //             current_cluster = next;
        //         },
        //         Status::Eoc(_) => {
        //             read += self.get_cluster_data(current_cluster, buf)?;
        //             break;
        //         },
        //         Status::Reserved => {
        //             return Err(io::Error::new(io::ErrorKind::InvalidData, "Reserved Space"));
        //         },
        //         Status::Free => {
        //             return Err(io::Error::new(io::ErrorKind::InvalidData, "Cluster is Free"));
        //         },
        //         Status::Bad => {
        //             return Err(io::Error::new(io::ErrorKind::InvalidData, "Bad Cluster"));
        //         },
        //         _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid entry")),
        //     }
        // }
        // Ok(read)

        let bytes_per_cluster = self.bytes_per_sector as usize * self.sectors_per_cluster as usize;
        let mut read = 0;
        let mut current = start;
        buf.clear();
        let mut cycle_detect = None;
        //check status of current fat entry
        match self.fat_entry( current )?.status() {
            Status::Data(x) => {
                cycle_detect = Some( x );
            },
            Status::Eoc(x) => {},
            _ => { return Err( io::Error::new( io::ErrorKind::InvalidData,
                                               "Invalid cluster chain 1" ) )
            },
        }
        loop {
            // println!("read chain loop");
            if let Some(x) = cycle_detect {
                if current.cluster_value() == x.cluster_value() {
                    return Err( io::Error::new( io::ErrorKind::InvalidData,
                                                "FAT cluster chain has a cycle" ) )
                }
            }
            buf.resize( read + bytes_per_cluster, 0 );
            let offset = 0;
            let bytes_read = self.read_cluster( current, offset, & mut buf[read..] )?;
            read += bytes_read;
            //advance to next cluster
            match self.fat_entry( current )?.status() {
                Status::Data( x ) => {
                    current = x;
                },
                Status::Eoc( x ) => {
                    break; //done
                },
                _ => { return Err( io::Error::new( io::ErrorKind::InvalidData,
                                                   "Invalid cluster chain 2" ) )
                },
            }
            //advance the cycle detector twice as fast
            for _ in 0..2 {
                if let Some( x ) = cycle_detect {
                    match self.fat_entry( x )?.status() {
                        Status::Data( y ) => {
                            cycle_detect = Some( y );
                        },
                        Status::Eoc(_) => {
                            cycle_detect = None;
                        },
                        _ => { return Err( io::Error::new( io::ErrorKind::InvalidData,
                                                           "Invalid cluster chain 3" ) )
                        },
                    }
                }
            }
        }
        Ok(read)
    }

    // reads the cluster into the buf, changes its length to stop from needing to copy
    // pub fn get_cluster_data(&mut self, cluster: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
    //     let size = self.bytes_per_sector as usize * self.sectors_per_cluster as usize;
    //     buf.reserve(size);
    //     let length = buf.len();
    //     unsafe {buf.set_len(length + size)};
    //     let read = self.read_cluster(cluster, 0, &mut buf[length..])?;
    //     unsafe {buf.set_len(length + read)}
    //     Ok(read)
    // }
    //
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
    pub fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
        // let fat_sector = (cluster.cluster_value() as u64 * size_of::<FatEntry>() as u64) 
        //  / self.bytes_per_sector as u64;
        // let offset = cluster.cluster_value() as usize * size_of::<FatEntry>() % self.bytes_per_sector as usize;
        // let content = self.device.get(self.fat_start_sector + fat_sector)?;
        // let entries: &[FatEntry] = unsafe { content.cast() };
        // Ok(&entries[offset / size_of::<FatEntry>()])
        use core::slice;
        let size = size_of::<FatEntry>();
        let origin = self.fat_start_sector;
        let sector_whole = (cluster.cluster_value() as usize) * (size as usize) / self.bytes_per_sector as usize;
        let remainder = (cluster.cluster_value() as usize) * size % self.bytes_per_sector as usize;
        let offset = origin as usize + sector_whole;
        let slice: &[u8] = self.device.get(offset as u64)?;
        let fat_entry = unsafe {
            slice::from_raw_parts((&slice[remainder] as * const u8) as * const FatEntry, 1)
        };
        Ok(&fat_entry[0])
    }

    pub fn get_cluster_data(&mut self, cluster: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        let size = self.bytes_per_sector as usize * self.sectors_per_cluster as usize;
        buf.reserve(size);
        let length = buf.len();
        unsafe {buf.set_len(length + size)};
        let read = self.read_cluster(cluster, 0, &mut buf[length..])?;
        unsafe {buf.set_len(length + read)}
        Ok(read)
    }

    pub fn get_cluster_size(&mut self) -> usize {
        return self.bytes_per_sector as usize * self.sectors_per_cluster as usize;
    }

    pub fn get_root_cluster(&mut self) -> Cluster {
        self.rootdir_cluster
    }

    pub fn get_bytes_per_sector(&mut self) -> usize{
        self.bytes_per_sector as usize
    }
}



impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Entry = Entry<HANDLE>;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
    //     use shim::path::Component;
 
    //     let path_ref = path.as_ref();
    //     if !path_ref.is_absolute() {
    //         return Err(io::Error::new(io::ErrorKind::InvalidInput, "path must be absolute"));
    //     }
 
    //     let mut v = Vec::new();
    //     for c in path_ref.components() {
    //         match c {
    //             Component::RootDir => {
    //                 v.clear();
    //                 let d = Dir::new(self.clone());
    //                 {
    //                     // use traits::Dir;
    //                     v.push( Entry::DirEntry(d) );
    //                 }
    //             },
    //             Component::Normal(x) => {
    //                 use crate::traits::Entry;
 
    //                 let new_entry = match v.last() {
    //                     Some(current_entry) =>
    //                         match current_entry.as_dir() {
    //                             Some(dir) => dir.find(x)?, // find this name in the normal component
    //                             None => { //unimplemented!()
    //                                 return Err(io::Error::new(io::ErrorKind::NotFound, "file not found"));
    //                             }
    //                     },
    //                     None => return Err(io::Error::new(
    //                         io::ErrorKind::NotFound,
    //                         "file not found",
    //                     ))
    //                 };
    //                 v.push(new_entry);
    //             },
    //             Component::ParentDir => { v.pop(); },
    //             Component::CurDir => {},
    //             _ => { unimplemented!(); },
    //         }
    //     }
 
    //     match v.into_iter().last() {
    //         Some(x) => { Ok(x) },
    //         _ => {Err(io::Error::new(io::ErrorKind::NotFound, "file cannot be found"))},
    //     }
    // }
        use shim::path::Component;
        let p = path.as_ref();
        if !p.is_absolute() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "path is not absolute"));
        }
        let mut vec = Vec::new();
        for component in p.components() {
            match component {
                Component::RootDir => {
                    vec.clear();
                    let d = Dir::new(self.clone());
                    vec.push(Entry::DirEntry(d));
                },
                Component::Normal(name) => {
                    let entry = if let Some(entry) = vec.last() {
                        use crate::traits::Entry;
                        if let Some(dir) = entry.as_dir() {
                            Some(dir.find(name)?)
                        } else {
                            None
                        }
                    } else {
                        None                 
                    };
                    match entry {
                        Some(e) => {
                            vec.push(e);
                        },
                        None => {
                            return Err(io::Error::new(io::ErrorKind::NotFound, "file cannot be found"));
                        }
                    }
                },
                Component::ParentDir => {vec.pop();},
                _ => (),
            }
        }
        match vec.into_iter().last() {
            Some(e) => Ok(e),
            _ => Err(io::Error::new(io::ErrorKind::NotFound, "file cannot be found")),
        }

        // use crate::traits::Entry;
        // use crate::alloc::string::ToString;
        // let components = path.as_ref().components();
        // let mut dir_entries: Vec<crate::vfat::Entry<HANDLE>> = Vec::new();
        // for c in components {
        //     match c {
        //         Component::RootDir => {
        //             dir_entries.truncate(0);
        //             dir_entries.push(crate::vfat::Entry::DirEntry(Dir {
        //                 vfat: self.clone(),
        //                 metadata: Metadata::default(),
        //                 first_cluster: self.lock(|vfat| vfat.rootdir_cluster),
        //                 name: "root".to_string()
        //             }));
        //         }
        //         Component::Normal(name) => {
        //             let new_entry = match dir_entries.last() {
        //                 Some(curr_entry) => {
        //                     match curr_entry.as_dir() {
        //                         Some(dir) => dir.find(name)?,
        //                         None => return ioerr!(NotFound, "file not found")
        //                     }
        //                 },
        //                 None => return ioerr!(NotFound, "file not found")
        //             };
        //             dir_entries.push(new_entry);
        //         },
        //         Component::ParentDir => {
        //             if dir_entries.len() > 0 {
        //                 dir_entries.pop();
        //             } else {
        //                 return ioerr!(NotFound, "file not found");
        //             }
        //         },
        //         _ => (),
        //     }
        // }
        // let entry = match dir_entries.into_iter().last() {
        //     Some(ent) => ent,
        //     None => return ioerr!(NotFound, "file not found")
        // };
        // Ok(entry)
    }
}
