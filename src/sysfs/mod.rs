mod utils;

pub(crate) use utils::{sysfs_parse_list, sysfs_read_file};

pub type Result<T> = std::result::Result<T, SysfsError>;

#[derive(Debug, thiserror::Error)]
pub enum SysfsError {
    /// Kernel documentation says that if you get os error 2 that
    /// means a feature is unavailable.
    #[error("the requested sysfs attribute does not exist")]
    MissingAttribute,
    #[error("encountered IO error: {0}")]
    Io(#[from] std::io::Error),
}
