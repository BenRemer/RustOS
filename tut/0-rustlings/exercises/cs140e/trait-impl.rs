// FIXME: Make me pass! Diff budget: 25 lines.
#![feature(type_alias_enum_variants)]

use Duration::*;
#[derive(Debug)]
enum Duration {
    MilliSeconds(u64),
    Seconds(u32),
    Minutes(u16),
}
// What traits does `Duration` need to implement?
impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        let mut l = self::MilliSeconds as u64 + self::Seconds as u64 * 1000 + self::Minutes as u64 * 60000;
        let mut r = Self::MilliSeconds as u64 + Self::Seconds as u64 * 1000 + Self::Minutes as u64 * 60000;
        l == r
    }
}

#[test]
fn traits() {
    assert_eq!(Duration::Seconds(120), Duration::Minutes(2));
    assert_eq!(Duration::Seconds(420), Duration::Minutes(7));
    assert_eq!(Duration::MilliSeconds(420000), Duration::Minutes(7));
    assert_eq!(Duration::MilliSeconds(43000), Duration::Seconds(43));
}
