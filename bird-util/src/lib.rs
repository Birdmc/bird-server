pub struct ConstAssert<const EXPR: bool>;

pub trait ConstAssertTrue {}

pub trait ConstAssertFalse {}

impl ConstAssertTrue for ConstAssert<true> {}

impl ConstAssertFalse for ConstAssert<false> {}