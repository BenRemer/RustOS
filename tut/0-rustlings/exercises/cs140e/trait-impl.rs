// FIXME: Make me pass! Diff budget: 25 lines.
#![feature(type_alias_enum_variants)]

<<<<<<< HEAD
use Duration::*;
#[derive(Debug)]
=======
// I AM NOT DONE

>>>>>>> skeleton/lab2
enum Duration {
    MilliSeconds(u64),
    Seconds(u32),
    Minutes(u16)
}
<<<<<<< HEAD
// What traits does `Duration` need to implement?
impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        let mut l = self::MilliSeconds as u64 + self::Seconds as u64 * 1000 + self::Minutes as u64 * 60000;
        let mut r = Self::MilliSeconds as u64 + Self::Seconds as u64 * 1000 + Self::Minutes as u64 * 60000;
        l == r
    }
}
=======
>>>>>>> skeleton/lab2

#[test]
fn traits() {
    assert_eq!(Seconds(120), Minutes(2));
    assert_eq!(Seconds(420), Minutes(7));
    assert_eq!(MilliSeconds(420000), Minutes(7));
    assert_eq!(MilliSeconds(43000), Seconds(43));
}
