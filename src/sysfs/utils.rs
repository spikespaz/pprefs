// <https://www.kernel.org/doc/html/latest/filesystems/sysfs.html>
// <https://www.kernel.org/doc/html/latest/admin-guide/sysfs-rules.html>
//
// If you see unchecked string functions being called,
// it's because *sysfs* is guaranteed to be ASCII (where we expect text).

// The maximum number of bytes that can be read from any given
// *sysfs* attribute. Generally there should be nothing larger than this.

pub const SYSFS_MAX_ATTR_BYTES: usize = 1024;

macro_rules! impl_sysfs_attrs {
    () => {};
    (
        $(#[$meta:meta])*
        $vis:vis sysfs_attr $attr_name:ident ($($arg_ident:ident : $arg_ty:ty),*)
        in $sysfs_dir:literal {
            read: $read_op:expr => $read_ty:ty,
            $(write: $write_op:expr,)?
        }

        $($tail:tt)*
    ) => {
        $crate::sysfs::impl_sysfs_read!(
            $(#[$meta])*
            $vis fn $attr_name ($($arg_ident : $arg_ty),*)
                in $sysfs_dir
                for $read_op => $read_ty;
        );

        $crate::sysfs::impl_sysfs_attrs!($($tail)*);
    };
}

pub(crate) use impl_sysfs_attrs;

/// UNSAFE
macro_rules! impl_sysfs_read {
    () => {};
    (
        $(#[$meta:meta])*
        $vis:vis fn $attr_name:ident ( $($arg:ident : $arg_ty:ty),* )
            in $sysfs_dir:literal
            match {
                $($arm_pat:pat $(if $arm_cond:expr)? => $arm_expr:expr),+
            } => $result_ty:ty;
    ) => {
        $(#[$meta])*
        $vis fn $attr_name($($arg: $arg_ty,)*) -> $result_ty {
            use std::fs::OpenOptions;
            use std::io::{ErrorKind, Read};

            use $crate::sysfs::{SysfsError, SYSFS_MAX_ATTR_BYTES};

            let file_path = &format!("{}/{}", format_args!($sysfs_dir), stringify!($attr_name));

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

            #[allow(clippy::redundant_closure_call)]
            match result {
                $($arm_pat $(if $arm_cond)? => $arm_expr),+
            }
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis fn $attr_name:ident ( $($arg:ident : $arg_ty:ty),* )
            in $sysfs_dir:literal
            for $parse_ok:expr => $ok_ty:ty;
    ) => {
        $crate::sysfs::impl_sysfs_read!(
            $(#[$meta])*
            $vis fn $attr_name($($arg: $arg_ty),*)
            in $sysfs_dir
            match {
                Ok(text) if text == "<unsupported>" => Err(SysfsError::UnsupportedAttribute),
                Ok(text) => Ok($parse_ok(text)),
                Err(e) if e.kind() == ErrorKind::NotFound => Err(SysfsError::MissingAttribute),
                Err(e) => Err(SysfsError::from(e))
            } => $crate::sysfs::Result<$ok_ty>;
        );
    };
}

pub(crate) use impl_sysfs_read;
