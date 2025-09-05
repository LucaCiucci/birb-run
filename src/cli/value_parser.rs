
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomValueParser<T>(std::marker::PhantomData<T>);

impl<T> CustomValueParser<T> {
    pub fn new() -> Self {
        CustomValueParser(std::marker::PhantomData)
    }
}