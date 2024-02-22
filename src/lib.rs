use tracing_appender::rolling::RollingFileAppender;

pub mod config;
pub mod domain;
pub mod file_datasource;

/// A trait for applying Kotlin-like convenience methods to types.
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

// Implement the trait for all types.
impl<T> KtConvenience for T {}

/// A macro for cloning variables. Useful for moving
/// variables into closures, like `Rc` or `Arc`.
#[macro_export]
macro_rules! reclone {
    ($($v:ident),+ $(,)?) => {
        $(
            let $v = $v.clone();
        )+
    }
}

/// Struct, used for logging to both file and stdout.
pub struct FileStdoutWriter {
    file: RollingFileAppender,
}

impl FileStdoutWriter {
    pub fn new(file: RollingFileAppender) -> Self {
        Self { file }
    }
}

impl std::io::Write for FileStdoutWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let buf_len = buf.len();
        std::io::stdout().write(buf)?;
        self.file.write(buf)?;
        Ok(buf_len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()?;
        self.file.flush()
    }
}
