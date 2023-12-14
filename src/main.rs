// <https://github.com/torvalds/linux/blob/master/tools/power/cpupower/utils/helpers/sysfs.c>

pub mod sysfs;

/// <https://www.kernel.org/doc/html/latest/admin-guide/pm/cpufreq.html?highlight=schedutil#policy-interface-in-sysfs>
pub mod cpufreq {
    use std::os::unix::ffi::OsStrExt;

    use crate::sysfs::{sysfs_parse_list, sysfs_read_file, Result};

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
        // Unsafe is safe because sysfs is all ASCII text.
        // Unwrap is safe because the documentation guarantees a list of space-separated ASCII numbers.
        unsafe {
            Ok(sysfs_parse_list(&sysfs_read_file(&format!(
                "{}/policy{}/affected_cpus",
                SYSFS_DIR, cpu_num
            ))?))
        }
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
