pub use sysfs_lib as lib;
pub use sysfs_lib::{Error, Result};

extern crate self as sysfs;

mod api;
pub use api::*;
