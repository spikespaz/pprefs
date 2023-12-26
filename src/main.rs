// <https://github.com/torvalds/linux/blob/master/tools/power/cpupower/utils/helpers/sysfs.c>

use sysfs_cpu as cpu;
use sysfs_cpu::cpufreq;

fn main() {
    for cpu_num in 0..cpu::num_cpus().unwrap() {
        println!(
            r#"/sys/devices/system/cpu/cpufreq/policy{}:
    affected_cpus               - {:?}
    bios_limit                  - {:?}
    cpuinfo_cur_freq            - {:?}
    cpuinfo_max_freq            - {:?}
    cpuinfo_min_freq            - {:?}
    cpuinfo_transition_latency  - {:?}
    related_cpus                - {:?}
    scaling_available_governors - {:?}
    scaling_cur_freq            - {:?}
    scaling_driver              - {:?}
    scaling_governor            - {:?}
    scaling_max_freq            - {:?}
    scaling_setspeed            - {:?}
"#,
            cpu_num,
            cpufreq::affected_cpus(cpu_num),
            cpufreq::bios_limit(cpu_num),
            cpufreq::cpuinfo_cur_freq(cpu_num),
            cpufreq::cpuinfo_max_freq(cpu_num),
            cpufreq::cpuinfo_min_freq(cpu_num),
            cpufreq::cpuinfo_transition_latency(cpu_num),
            cpufreq::related_cpus(cpu_num),
            cpufreq::scaling_available_governors(cpu_num),
            cpufreq::scaling_cur_freq(cpu_num),
            cpufreq::scaling_driver(cpu_num),
            cpufreq::scaling_governor(cpu_num),
            cpufreq::scaling_max_freq(cpu_num),
            cpufreq::scaling_setspeed(cpu_num),
        )
    }
}
