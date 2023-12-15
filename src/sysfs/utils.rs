// <https://www.kernel.org/doc/html/latest/filesystems/sysfs.html>
// <https://www.kernel.org/doc/html/latest/admin-guide/sysfs-rules.html>
//
// If you see unchecked string functions being called,
// it's because *sysfs* is guaranteed to be ASCII (where we expect text).

// The maximum number of bytes that can be read from any given
// *sysfs* attribute. Generally there should be nothing larger than this.

pub const SYSFS_MAX_ATTR_BYTES: usize = 1024;

/// UNSAFE
macro_rules! impl_sysfs_read {
    (
        $(#[$meta:meta])*
        $vis:vis fn $attr_name:ident ( $($arg:ident : $arg_ty:ty),* ) in $sysfs_dir:literal -> $ret_ty:tt;
    ) => {
        // Allowed because blah blah metavariable expansion syntax error blah blah
        #[allow(unused_parens)]
        $vis fn $attr_name($($arg: $arg_ty,)*) -> $crate::sysfs::Result<$ret_ty> {
            use std::fs::OpenOptions;
            use std::io::{ErrorKind, Read};

            use $crate::sysfs::{impl_sysfs_read, SysfsError, SYSFS_MAX_ATTR_BYTES};

            let file_path = &format!("{}/{}", format_args!($sysfs_dir), stringify!($attr_name));
            let mut buf = [0; SYSFS_MAX_ATTR_BYTES];
            let result = OpenOptions::new()
                .read(true)
                .open(file_path)
                .and_then(|mut f| {
                    let bytes_read = f.read(&mut buf)?;
                    // SAFETY: Linux guarantees that all of *sysfs* is valid ASCII.
                    let buf = unsafe { std::str::from_utf8_unchecked(&buf[..bytes_read]) };
                    // Unchecked conversion is safe because this attribute is ASCII.
                    let buf = buf.trim_end_matches('\n');
                    Ok(buf.to_owned())
                })
                .map_err(|e| {
                    if e.kind() == ErrorKind::NotFound {
                        SysfsError::MissingAttribute
                    } else {
                        SysfsError::from(e)
                    }
                });

            impl_sysfs_read!(result, $ret_ty)
        }
    };
    ($result:expr, usize) => {{
        Ok($result?.parse().unwrap())
    }};
    ($result:expr, (Vec< $t:tt >)) => {{
        Ok($result?.split(' ').map(|item| item.parse().unwrap()).collect())
    }}
}

pub(crate) use impl_sysfs_read;
