pub mod domain;
pub mod config;
pub mod file_datasource;

pub trait KtConvenience: Sized {
    fn apply(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    fn r#let<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> KtConvenience for T {}

#[macro_export]
macro_rules! reclone {
    ($($v:ident),+ $(,)?) => {
        $(
            let $v = $v.clone();
        )+
    }
}
