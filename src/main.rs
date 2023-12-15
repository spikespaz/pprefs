// <https://github.com/torvalds/linux/blob/master/tools/power/cpupower/utils/helpers/sysfs.c>

pub mod sysfs;

use sysfs::cpufreq;

fn main() {
    for cpu_num in 0..cpufreq::num_policies().unwrap() {
        println!(
            "policy{}:\naffected_cpus = {:?}",
            cpu_num,
            cpufreq::affected_cpus(cpu_num).unwrap()
        )
    }
}
