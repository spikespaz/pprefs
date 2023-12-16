// <https://www.kernel.org/doc/html/latest/filesystems/sysfs.html>
// <https://www.kernel.org/doc/html/latest/admin-guide/sysfs-rules.html>
//
// If you see unchecked string functions being called,
// it's because *sysfs* is guaranteed to be ASCII (where we expect text).

// The maximum number of bytes that can be read from any given
// *sysfs* attribute. Generally there should be nothing larger than this.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Read as _};

use super::{Result, SysfsError};

pub const SYSFS_MAX_ATTR_BYTES: usize = 1024;

pub(crate) fn sysfs_read<T>(file_path: &str, parse_ok: fn(&str) -> T) -> Result<T> {
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
            $crate::sysfs::sysfs_read::< $read_ty >(&file_path, $parse_ok)
        }

        $crate::sysfs::impl_sysfs_attrs!($($tail)*);
    };
}

pub(crate) use impl_sysfs_attrs;
