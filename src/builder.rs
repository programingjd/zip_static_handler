use crate::errors::Result;
use crate::handler::Handler;
use std::borrow::Borrow;
use std::marker::PhantomData;

pub trait ZipPrefix {
    fn zip_prefix(self) -> Option<String>;
}
pub trait WithoutZipPrefix: ZipPrefix {}
pub trait WithZipPrefix: ZipPrefix {}
impl ZipPrefix for () {
    fn zip_prefix(self) -> Option<String> {
        None
    }
}
impl WithoutZipPrefix for () {}
impl ZipPrefix for String {
    fn zip_prefix(self) -> Option<String> {
        Some(self)
    }
}
impl WithZipPrefix for String {}
pub trait RootPrefix {
    fn root_prefix(self) -> Option<String>;
}
pub trait WithoutRootPrefix: RootPrefix {}
pub trait WithRootPrefix: RootPrefix {}
impl RootPrefix for () {
    fn root_prefix(self) -> Option<String> {
        None
    }
}
impl WithoutRootPrefix for () {}
impl RootPrefix for String {
    fn root_prefix(self) -> Option<String> {
        Some(self)
    }
}
impl WithRootPrefix for String {}
pub trait Diff<'a> {
    fn diff(self) -> Option<&'a Handler>;
}
pub trait WithoutDiff<'a>: Diff<'a> {}
pub trait WithDiff<'a>: Diff<'a> {}
impl<'a> Diff<'a> for () {
    fn diff(self) -> Option<&'a Handler> {
        None
    }
}
impl<'a> WithoutDiff<'a> for () {}
impl<'a> Diff<'a> for &'a Handler {
    fn diff(self) -> Option<&'a Handler> {
        Some(self)
    }
}
impl<'a> WithDiff<'a> for &'a Handler {}
pub trait Bytes {}
pub trait WithoutBytes: Bytes {}
pub struct NoBytes;
impl Bytes for NoBytes {}
impl WithoutBytes for NoBytes {}
impl<T: Borrow<[u8]>> Bytes for T {}

pub struct Builder<'a, Z: ZipPrefix, R: RootPrefix, D: Diff<'a>, B: Bytes> {
    _lifetime: PhantomData<&'a ()>,
    zip_prefix: Z,
    root_prefix: R,
    diff: D,
    bytes: B,
}

impl Handler {
    pub fn builder() -> Builder<'static, (), (), (), NoBytes> {
        Builder {
            _lifetime: PhantomData,
            zip_prefix: (),
            root_prefix: (),
            diff: (),
            bytes: NoBytes,
        }
    }
}

impl<'a, Z: WithoutZipPrefix, R: RootPrefix, D: Diff<'a>, B: Bytes> Builder<'a, Z, R, D, B> {
    pub fn with_zip_prefix(self, prefix: impl Into<String>) -> Builder<'a, String, R, D, B> {
        Builder {
            _lifetime: PhantomData,
            zip_prefix: prefix.into(),
            root_prefix: self.root_prefix,
            diff: self.diff,
            bytes: self.bytes,
        }
    }
}

impl<'a, Z: ZipPrefix, R: WithoutRootPrefix, D: Diff<'a>, B: Bytes> Builder<'a, Z, R, D, B> {
    pub fn with_root_prefix(self, prefix: impl Into<String>) -> Builder<'a, Z, String, D, B> {
        Builder {
            _lifetime: PhantomData,
            zip_prefix: self.zip_prefix,
            root_prefix: prefix.into(),
            diff: self.diff,
            bytes: self.bytes,
        }
    }
}

impl<'a, Z: ZipPrefix, R: RootPrefix, D: WithDiff<'a>, B: Bytes> Builder<'a, Z, R, D, B> {
    pub fn with_diff(self, diff: &'a Handler) -> Builder<'a, Z, R, &'a Handler, B> {
        Builder {
            _lifetime: PhantomData,
            zip_prefix: self.zip_prefix,
            root_prefix: self.root_prefix,
            diff,
            bytes: self.bytes,
        }
    }
}

impl<'a, Z: ZipPrefix, R: RootPrefix, D: Diff<'a>, B: WithoutBytes> Builder<'a, Z, R, D, B> {
    pub fn with_zip<T: Borrow<[u8]>>(self, bytes: T) -> Builder<'a, Z, R, D, T> {
        Builder {
            _lifetime: PhantomData,
            zip_prefix: self.zip_prefix,
            root_prefix: self.root_prefix,
            diff: self.diff,
            bytes,
        }
    }
}

impl<'a, Z: ZipPrefix, R: RootPrefix, D: Diff<'a>, B: Borrow<[u8]>> Builder<'a, Z, R, D, B> {
    pub fn try_build(self) -> Result<Handler> {
        Handler::try_new(
            self.bytes,
            self.root_prefix.root_prefix().unwrap_or_default(),
            self.zip_prefix.zip_prefix().unwrap_or_default(),
            self.diff.diff(),
        )
    }
}
