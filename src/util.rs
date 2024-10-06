use std::time::{SystemTime, UNIX_EPOCH};


/// Crude sequence generator
/// Linear congruent with Hull-Dobel theorem
/// Shouldn't generate any overlapping numbers (period is 2^64)
pub struct UUIDGen{
    prev: u64
}
impl UUIDGen{
    pub fn new(seed: u64)->Self{
        Self { prev: seed ^ 0x1234567890ABCDEF } // Sugar it a bit
    }
    pub fn new_now()->Self{ Self::new(get_time_millis()) }
    pub fn next(&mut self) -> u64{
        const A: u64 = 8388356123327754055;
        const C: u64 = 1; // If C is 1 I think that satisfies all conditions
        self.prev = self.prev.wrapping_mul(A).wrapping_add(C);
        return self.prev;
    }
}

pub fn get_time_millis()->u64{
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

#[macro_export]
macro_rules! fi {
    ($condition:expr, $true_case:expr, $false_case:expr) => {
        if $condition { $true_case } else { $false_case }
    };
}