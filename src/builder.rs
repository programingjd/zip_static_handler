use crate::errors::Result;
use crate::handler::{Handler, HeaderSelector};
use crate::types::DefaultHeaderSelector;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::io::Cursor;
use std::marker::PhantomData;
use tracing::trace;
use zip_structs::zip_central_directory::ZipCDEntry;
use zip_structs::zip_eocd::ZipEOCD;

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
pub trait PathPrefix {
    fn path_prefix(self) -> Option<String>;
}
pub trait WithoutPathPrefix: PathPrefix {}
pub trait WithPathPrefix: PathPrefix {}
impl PathPrefix for () {
    fn path_prefix(self) -> Option<String> {
        None
    }
}
impl WithoutPathPrefix for () {}
impl PathPrefix for String {
    fn path_prefix(self) -> Option<String> {
        Some(self)
    }
}
impl WithPathPrefix for String {}
pub trait CustomHeaderSelector<'a> {
    fn header_selector(self) -> Option<&'a dyn HeaderSelector>;
}
pub trait WithCustomHeaderSelector<'a>: CustomHeaderSelector<'a> {}
pub trait WithoutCustomHeaderSelector<'a>: CustomHeaderSelector<'a> {}
impl<'a> CustomHeaderSelector<'a> for () {
    fn header_selector(self) -> Option<&'a dyn HeaderSelector> {
        None
    }
}
impl<'a> WithoutCustomHeaderSelector<'a> for () {}
impl<'a, T: HeaderSelector> WithCustomHeaderSelector<'a> for &'a T {}
impl<'a, T: HeaderSelector> CustomHeaderSelector<'a> for &'a T {
    fn header_selector(self) -> Option<&'a dyn HeaderSelector> {
        Some(self)
    }
}
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

pub struct Builder<
    'a,
    'b,
    Z: ZipPrefix,
    R: PathPrefix,
    H: CustomHeaderSelector<'a>,
    D: Diff<'b>,
    B: Bytes,
> {
    _a: PhantomData<&'a ()>,
    _b: PhantomData<&'b ()>,
    zip_prefix: Z,
    path_prefix: R,
    header_selector: H,
    diff: D,
    bytes: B,
}

impl Handler {
    pub fn builder() -> Builder<'static, 'static, (), (), (), (), NoBytes> {
        Builder {
            _a: PhantomData,
            _b: PhantomData,
            zip_prefix: (),
            path_prefix: (),
            header_selector: (),
            diff: (),
            bytes: NoBytes,
        }
    }
}

impl<
        'a,
        'b,
        Z: WithoutZipPrefix,
        R: PathPrefix,
        H: CustomHeaderSelector<'a>,
        D: Diff<'b>,
        B: Bytes,
    > Builder<'a, 'b, Z, R, H, D, B>
{
    pub fn with_zip_prefix(self, prefix: impl Into<String>) -> Builder<'a, 'b, String, R, H, D, B> {
        Builder {
            _a: PhantomData,
            _b: PhantomData,
            zip_prefix: prefix.into(),
            path_prefix: self.path_prefix,
            header_selector: self.header_selector,
            diff: self.diff,
            bytes: self.bytes,
        }
    }
}

impl<
        'a,
        'b,
        Z: ZipPrefix,
        R: WithoutPathPrefix,
        H: CustomHeaderSelector<'a>,
        D: Diff<'b>,
        B: Bytes,
    > Builder<'a, 'b, Z, R, H, D, B>
{
    pub fn with_root_prefix(
        self,
        prefix: impl Into<String>,
    ) -> Builder<'a, 'b, Z, String, H, D, B> {
        Builder {
            _a: PhantomData,
            _b: PhantomData,
            zip_prefix: self.zip_prefix,
            path_prefix: prefix.into(),
            header_selector: self.header_selector,
            diff: self.diff,
            bytes: self.bytes,
        }
    }
}

impl<
        'a,
        'b,
        Z: ZipPrefix,
        R: PathPrefix,
        H: WithoutCustomHeaderSelector<'a>,
        D: Diff<'b>,
        B: Bytes,
    > Builder<'a, 'b, Z, R, H, D, B>
{
    pub fn with_custom_header_selector<S: WithCustomHeaderSelector<'a>>(
        self,
        header_selector: S,
    ) -> Builder<'a, 'b, Z, R, S, D, B> {
        Builder {
            _a: PhantomData,
            _b: PhantomData,
            zip_prefix: self.zip_prefix,
            path_prefix: self.path_prefix,
            header_selector,
            diff: self.diff,
            bytes: self.bytes,
        }
    }
}

impl<
        'a,
        'b,
        Z: ZipPrefix,
        R: PathPrefix,
        H: CustomHeaderSelector<'a>,
        D: WithoutDiff<'b>,
        B: Bytes,
    > Builder<'a, 'b, Z, R, H, D, B>
{
    pub fn with_diff(self, diff: &'b Handler) -> Builder<'a, 'b, Z, R, H, &'b Handler, B> {
        Builder {
            _a: PhantomData,
            _b: PhantomData,
            zip_prefix: self.zip_prefix,
            path_prefix: self.path_prefix,
            header_selector: self.header_selector,
            diff,
            bytes: self.bytes,
        }
    }
}

impl<
        'a,
        'b,
        Z: ZipPrefix,
        R: PathPrefix,
        H: CustomHeaderSelector<'a>,
        D: Diff<'b>,
        B: WithoutBytes,
    > Builder<'a, 'b, Z, R, H, D, B>
{
    pub fn with_zip<T: Borrow<[u8]>>(self, bytes: T) -> Builder<'a, 'b, Z, R, H, D, T> {
        Builder {
            _a: PhantomData,
            _b: PhantomData,
            zip_prefix: self.zip_prefix,
            path_prefix: self.path_prefix,
            header_selector: self.header_selector,
            diff: self.diff,
            bytes,
        }
    }
}

impl<
        'a,
        'b,
        Z: ZipPrefix,
        R: PathPrefix,
        H: CustomHeaderSelector<'a>,
        D: Diff<'b>,
        B: Borrow<[u8]>,
    > Builder<'a, 'b, Z, R, H, D, B>
{
    pub fn try_build(self) -> Result<Handler> {
        let bytes = self.bytes;
        let path_prefix = self.path_prefix.path_prefix().unwrap_or_default();
        let zip_prefix = self.zip_prefix.zip_prefix().unwrap_or_default();
        let diff = self.diff.diff();
        let header_selector = self
            .header_selector
            .header_selector()
            .unwrap_or(&DefaultHeaderSelector);
        trace!(path_prefix = path_prefix, zip_prefix = zip_prefix);
        let mut cursor = Cursor::new(bytes.borrow());
        let directory = ZipEOCD::from_reader(&mut cursor)?;
        let mut routes = HashMap::new();
        let entries = ZipCDEntry::all_from_eocd(&mut cursor, &directory)?;
        for entry in &entries {
            if let Some((path, value)) = crate::handler::build_entry(
                &mut cursor,
                zip_prefix.as_str(),
                entry,
                &entries,
                header_selector,
                diff,
            )? {
                if path.ends_with('/') && path.len() > 1 {
                    let no_trailing_slash = &path[..path.len() - 1];
                    routes.insert(
                        format!("{path_prefix}{path}"),
                        crate::handler::redirect_entry(no_trailing_slash),
                    );
                    routes.insert(format!("{path_prefix}{no_trailing_slash}"), value);
                } else {
                    routes.insert(format!("{path_prefix}{path}"), value);
                }
            }
        }
        Ok(Handler {
            files: routes,
            error_headers: header_selector.error_headers(),
        })
    }
}
