use core::fmt;
use shim::const_assert_size;
use core::mem;
use crate::traits::BlockDevice;
use crate::vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    // Fill me in.
    jmp: [u8; 3],
    oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub number_fats: u8,
    max_directories: u16,
    logical_sectors: u16,
    media_descriptor: u8,
    pub fat_sectors: u16,
    sectors_per_track: u16,
    number_of_heads: u16,
    hidden_sectors: u32,
    // ext
    logical_sectors_2: u32,
    pub fat_sectors_2: u32,
    flags: u16,
    version: u16,
    pub root_cluster: u32,
    fs_info: u16,
    backup_sector: u16,
    _r1: [u8; 12],
    drive_number: u8,
    _r2: u8,
    signature: u8,
    volume_serial: u32,
    volume_label: [u8; 11],
    identifier: [u8; 8],
    boot_code: [u8; 420],
    partition_signature: u16,
}

const_assert_size!(BiosParameterBlock, 512);

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut buf = [0u8; 512];
        device.read_sector(sector, &mut buf).map_err(|err| {Error::Io(err)})?;
        let edpb: BiosParameterBlock = unsafe {mem::transmute(buf)};
        if edpb.partition_signature != 0xAA55 {
            return Err(Error::BadSignature);
        }
        Ok(edpb)
    }

    pub fn data_start_sector(&self) -> u64 {
        return self.reserved_sectors as u64 + self.fat_sectors_2 as u64 * self.number_fats as u64
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BiosParameterBlock").finish()
    }
}

