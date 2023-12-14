pub mod utils;

pub(crate) use utils::sysfs_read_file;

pub use utils::SysfsError;

pub type Result<T> = std::result::Result<T, SysfsError>;
