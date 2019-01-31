pub trait NumericCast<T> {
    fn saturate(self) -> T;
    fn try_cast(self) -> Option<T>;
}

impl NumericCast<i32> for usize {
    fn saturate(self) -> i32 {
        self.min(i32::max_value() as usize) as i32
    }

    fn try_cast(self) -> Option<i32> {
        if self > i32::max_value() as usize {
            None
        } else {
            Some(self as i32)
        }
    }
}
