
/// Crude sequence generator
/// Linear congruent with Hull-Dobel theorem
/// Shouldn't generate any overlapping numbers (period is 2^64)
pub struct UUIDGen{
    prev: u64
}
impl UUIDGen{
    pub fn new()->Self{
        Self { prev: 0x4A4F6E6F47B24E5A }
    }
    pub fn next(&mut self) -> u64{
        let a = 2023;
        let c = 2025;
        self.prev = self.prev.wrapping_mul(a).wrapping_add(c);
        return self.prev;
    }
}

#[macro_export]
macro_rules! fi {
    ($condition:expr, $true_case:expr, $false_case:expr) => {
        if $condition { $true_case } else { $false_case }
    };
}