// <https://www.kernel.org/doc/html/latest/filesystems/sysfs.html>
// <https://www.kernel.org/doc/html/latest/admin-guide/sysfs-rules.html>
//
// If you see unchecked string functions being called,
// it's because *sysfs* is guaranteed to be ASCII (where we expect text).

// The maximum number of bytes that can be read from any given
// *sysfs* attribute. Generally there should be nothing larger than this.

use std::fmt::Display;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Read as _, Write as _};

use super::{Result, SysfsError};

pub const SYSFS_MAX_ATTR_BYTES: usize = 1024;

/// UNSAFE: This function assumes makes an assumption that the contents of
/// the file at `file_path` is valid UTF-8 (in fact, it is expected to be
/// ASCII). It is undefined behavior to use this function with file paths not
/// exposed through *sysfs*.
pub(crate) unsafe fn sysfs_read<T>(file_path: &str, parse_ok: fn(&str) -> T) -> Result<T> {
    let mut buf = [0; SYSFS_MAX_ATTR_BYTES];
    let result = OpenOptions::new()
        .read(true)
        .open(file_path)
        .and_then(|mut f| {
            let bytes_read = f.read(&mut buf)?;
            // SAFETY: Linux guarantees that all of *sysfs* is valid ASCII.
            let buf = unsafe { std::str::from_utf8_unchecked(&buf[..bytes_read]) };
            let buf = buf.trim_end_matches('\n');
            Ok(buf)
        });

    match result {
        Ok("<unsupported>") => Err(SysfsError::UnsupportedAttribute),
        Ok(text) => Ok(parse_ok(text)),
        Err(e) if e.kind() == ErrorKind::NotFound => Err(SysfsError::MissingAttribute),
        Err(e) => Err(SysfsError::from(e)),
    }
}

/// This is a low-level function which only expects that `V: Display`. It does
/// not validate the value after formatting, so ensure that the `value` has a
/// sufficient `Display` implementation that will produce a string appropriate
/// for the *sysfs* attribute.
pub(crate) fn sysfs_write<V>(file_path: &str, value: V) -> Result<()>
where
    V: Display,
{
    OpenOptions::new()
        .read(false)
        .write(true)
        .create(false)
        .open(file_path)
        .and_then(|mut f| write!(f, "{}", value))
        .map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                SysfsError::MissingAttribute
            } else {
                SysfsError::from(e)
            }
        })
}

/// UNSAFE: Path templates to `sysfs` attributes are expected to be hard-coded
/// and validated before end-user runtime.
macro_rules! impl_sysfs_attrs {
    () => {};
    (
        $(#[$attr_meta:meta])*
        $vis:vis sysfs_attr $attr_name:ident ($($arg_ident:ident : $arg_ty:ty),*)
        in $sysfs_dir:literal {
            $(#[$getter_meta:meta])*
            read: $parse_ok:expr => $read_ty:ty,
        // $(
        //     $(#[$setter_meta:meta])*
        //     write: $write_op:expr,
        // )?
        }

        $($tail:tt)*
    ) => {

        $(#[$attr_meta])*
        #[doc = ""]
        $(#[$getter_meta])*
        $vis fn $attr_name ($($arg_ident : $arg_ty),*) -> $crate::sysfs::Result<$read_ty> {
            let file_path = format!("{}/{}", format_args!($sysfs_dir), stringify!($attr_name));
            unsafe {
                $crate::sysfs::sysfs_read::< $read_ty >(&file_path, $parse_ok)
            }
        }

        $crate::sysfs::impl_sysfs_attrs!($($tail)*);
    };
}

pub(crate) use impl_sysfs_attrs;
