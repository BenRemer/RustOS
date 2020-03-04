#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone, Hash)]
pub struct Cluster(pub u32);

impl From<u32> for Cluster {
    fn from(raw_num: u32) -> Cluster {
        Cluster(raw_num & !(0xF << 28))
    }
}

// Implement any useful helper methods on `Cluster`.
impl Cluster {
    pub fn cluster_value(&self) -> u32 {
        self.0
    }

    pub fn cluster_offset(&self) -> u32 {
        self.0.saturating_sub(2)
    }

    pub fn is_valid(&self) -> bool {
        self.0 > 2
    }
}