macro_rules! boxed {
    () => (
        std::box::Box::new()
    );
    ($elem:expr; $n:expr) => (
       std::box::from_elem($elem, $n)
    );
    ($($x:expr),+ $(,)?) => (
        std::boxed::Box::new([$($x),+])
    );
}
pub(crate) use boxed;
