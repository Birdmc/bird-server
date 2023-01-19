pub struct ConstAssert<const EXPR: bool>;

pub trait ConstAssertTrue {}

pub trait ConstAssertFalse {}

impl ConstAssertTrue for ConstAssert<true> {}

impl ConstAssertFalse for ConstAssert<false> {}

pub const fn const_log2_ceil(value: u64) -> u64 {
    if value <= 1 { return 0; }
    let mut counter = 1;
    loop {
        if value <= (1 << counter) {
            return counter;
        }
        counter += 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn const_log2_ceil_test() {
        fn check(valf: f64, valu: u64) {
            assert_eq!(valf.log2().ceil() as u64, const_log2_ceil(valu), "Failed: {} {}", valf, valu);
        }

        for i in 0..10000 {
            check(i as f64, i)
        }
    }

}