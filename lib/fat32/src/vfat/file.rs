use alloc::string::String;
 
use shim::io::{self, SeekFrom};
 
use crate::traits;
use crate::vfat::{Cluster, Metadata, VFatHandle, Status};
use core::cmp::min;
 
#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    // Fill me in.
    pub first_cluster: Cluster,
    // pub name: String,
    // pub name_long: String,
    pub name: String,
    pub metadata: Metadata,
    pub size: u64,
    pub curr_offset: u64,
    pub curr_cluster: Option<Cluster>,
    pub cluster_size: u64,
    // pub file_ptr: u32,
}
 
impl<HANDLE: VFatHandle> File<HANDLE> { 
    // pub fn name(&self) -> &str {
    //     self.name.as_str()
    // }
 
    pub fn metadata(&self) -> &Metadata {
        return &self.metadata;
    }
}
 
// Implement `traits::File` (and its supertraits) for `File`.
impl<HANDLE: VFatHandle> traits::File for File<HANDLE> {
    /// Writes any buffered data to disk.
    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
 
    /// Returns the size of the file in bytes.
    fn size(&self) -> u64 {
        return self.size as u64;
    }
}
 
impl<HANDLE: VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.size == 0{
            return Ok(0)
        }
        let length = min(buf.len(), (self.size - self.curr_offset) as usize);
        // let mut offset = if self.cluster_size != 0 {
        //     (self.curr_offset % self.cluster_size) as usize
        // } else {
        //     self.curr_offset as usize
        // };
        
        let mut offset = (self.curr_offset % self.cluster_size) as usize;
        let mut remaining = length;
        let mut curr_cluster = self.curr_cluster;
        let mut read = 0;
        // let mut exit: bool = false;
        while remaining > 0 {
            let size = self.vfat.lock(|vfat| vfat.read_cluster(
                curr_cluster.unwrap(), offset, &mut buf[read..length],
            ).expect("I/O error"));
 
            if size == (self.cluster_size as usize - offset) {
                self.vfat.lock(|vfat| match vfat.fat_entry(curr_cluster.unwrap()).expect("Invalid Fat").status() {
                        Status::Eoc(_) => {
                            curr_cluster = None;
                            // exit = true;
                        },
                        Status::Data(next_cluster) => curr_cluster = Some(next_cluster),
                        _ => unimplemented!(), 
                    }
                )
            }
            read += size;
            remaining -= size;
            offset = 0;
            // if exit {
            //     break;
            // }
        }
        self.curr_offset += length as u64;
        self.curr_cluster = curr_cluster;
        Ok(length)
    }
}
 
impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unimplemented!();
    }
 
    fn flush(&mut self) -> io::Result<()> {
        unimplemented!();
    }
}
 
impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        let offset = match _pos {
            SeekFrom::Start(start) => start,
            SeekFrom::Current(offset) => self.curr_offset.wrapping_add(offset as u64),
            SeekFrom::End(offset) => self.size.wrapping_add(offset as u64),
        }; 
        if offset >= self.size {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid seek position"));
        } else {
            let mut curr_cluster = self.first_cluster;
            let end = offset / self.cluster_size;
            for _ in 0..end {
                self.vfat.lock(|vfat|
                    match vfat.fat_entry(curr_cluster).expect("Fat Entry not found").status() {
                        Status::Data(next_cluster) => {curr_cluster = next_cluster;},
                        _ => unimplemented!(),
                });               
            }
            self.curr_cluster = Some(curr_cluster);
            self.curr_offset = offset;
            return Ok(self.curr_offset as u64);
        }
    }
}
