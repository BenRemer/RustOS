use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;

#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    // Change names due to conflicting names
    // File(File<HANDLE>),
    // Dir(Dir<HANDLE>),
    FileEntry(File<HANDLE>),
    DirEntry(Dir<HANDLE>),
}

// Implement any useful helper methods on `Entry`.
impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    // Implement `traits::Entry` for `Entry`.
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    /// The name of the file or directory corresponding to this entry.
    fn name(&self) -> &str {
        match self {
            &Entry::FileEntry(ref file) => file.name.as_str(),
            &Entry::DirEntry(ref dir) => dir.name.as_str(),
        }
    }

    /// The metadata associated with the entry.
    fn metadata(&self) -> &Self::Metadata {
        match self {
            &Entry::FileEntry(ref file) => file.metadata(), 
            &Entry::DirEntry(ref dir) => dir.metadata(), 
        }
    }

    /// If `self` is a file, returns `Some` of a reference to the file.
    /// Otherwise returns `None`.
    fn as_file(&self) -> Option<&Self::File> {
        match self {
            &Entry::FileEntry(ref file) => Some(file),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of a reference to the
    /// directory. Otherwise returns `None`.
    fn as_dir(&self) -> Option<&Self::Dir> {
        match self {
            &Entry::DirEntry(ref dir) => Some(dir),
            _ => None,
        }
    }

    /// If `self` is a file, returns `Some` of the file. Otherwise returns
    /// `None`.
    fn into_file(self) -> Option<Self::File> {
        match self {
            Entry::FileEntry(file) => Some(file),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of the directory. Otherwise
    /// returns `None`.
    fn into_dir(self) -> Option<Self::Dir> {
        match self {
            Entry::DirEntry(dir) => Some(dir),
            _ => None,
        }
    }
}