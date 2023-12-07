// <https://www.kernel.org/doc/html/latest/filesystems/sysfs.html>

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("encountered IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod cpufreq {
    use std::os::unix::ffi::OsStrExt;

    use super::Result;

    static SYSFS_DIR: &str = "/sys/devices/system/cpu/cpufreq";

    pub fn num_policies() -> Result<usize> {
        let policy_prefix = "policy".as_bytes();
        std::fs::read_dir(SYSFS_DIR)?.try_fold(0, |acc, res| match (acc, res) {
            (acc, Ok(inode))
                if {
                    let name = inode.file_name();
                    let name = name.as_bytes();
                    // Not sure if this is robust enough.
                    // This will make sure that the name starts with "policy" but does not equal exactly "policy".
                    // It does not, however, check for non-numeric characters at the end of the "policy" prefix.
                    // Realistically, we need to pattern match here, something like `^policy[0-9]+$`.
                    // Unfortunately regex is heavier than I want it to be, so that's not a good option.
                    name.len() >= policy_prefix.len()
                        && &name[..policy_prefix.len()] == policy_prefix
                } =>
            {
                Ok(acc + 1)
            }
            (acc, Ok(_)) => Ok(acc),
            (_, Err(e)) => Err(e.into()),
        })
    }
}

fn main() {
    dbg!(cpufreq::num_policies());
}
