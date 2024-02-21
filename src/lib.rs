pub mod config;
pub mod domain;
pub mod file_datasource;

pub trait KtConvenience: Sized {
    #[inline]
    fn apply(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    #[inline]
    fn try_apply<E>(mut self, f: impl FnOnce(&mut Self) -> Result<(), E>) -> Result<Self, E> {
        f(&mut self)?;
        Ok(self)
    }

    #[inline]
    fn r#let<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }

    #[inline]
    fn take_if(self, predicate: impl FnOnce(&Self) -> bool) -> Option<Self> {
        if predicate(&self) {
            Some(self)
        } else {
            None
        }
    }

    #[inline]
    fn also(self, f: impl FnOnce(&Self)) -> Self {
        f(&self);
        self
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
