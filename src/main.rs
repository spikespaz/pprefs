// <https://www.kernel.org/doc/html/latest/filesystems/sysfs.html>
// <https://www.kernel.org/doc/html/latest/admin-guide/sysfs-rules.html>
//
// If you see unchecked string functions being called,
// it's because *sysfs* is guaranteed to be ASCII (where we expect text).

//! The maximum number of bytes that can be read from any given
//! *sysfs* attribute. Generally there should be nothing larger than this.

const SYSFS_MAX_ATTR_BYTES: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the requested sysfs attribute does not exist")]
    MissingAttribute,
    #[error("encountered IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// <https://www.kernel.org/doc/html/latest/admin-guide/pm/cpufreq.html?highlight=schedutil#policy-interface-in-sysfs>
pub mod cpufreq {
    use std::fs::OpenOptions;
    use std::io::Read;
    use std::os::unix::ffi::OsStrExt;

    use crate::SYSFS_MAX_ATTR_BYTES;

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

    pub fn affected_cpus(cpu_num: usize) -> Result<Vec<usize>> {
        let sysfs_attr = format!("{}/policy{}/affected_cpus", SYSFS_DIR, cpu_num);
        let mut file = OpenOptions::new().read(true).open(sysfs_attr)?;
        let mut buf = [0; SYSFS_MAX_ATTR_BYTES];
        let bytes_read = file.read(&mut buf)?;
        // Unchecked conversion is safe because this attribute is ASCII.
        let content = unsafe { String::from_utf8_unchecked(Vec::from(&buf[..bytes_read])) };
        let content = content.trim_end_matches('\n'); // I do not think this should be necessary, but evidently it is.
                                                      // Unwrap is safe because the documentation guarantees a list of space-separated ASCII numbers.
        Ok(content.split(' ').map(|cpu| cpu.parse().unwrap()).collect())
    }
}

fn main() {
    for cpu_num in 0..cpufreq::num_policies().unwrap() {
        println!(
            "policy{}:\naffected_cpus = {:?}",
            cpu_num,
            cpufreq::affected_cpus(cpu_num).unwrap()
        )
    }
}
