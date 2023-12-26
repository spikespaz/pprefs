//! <https://www.kernel.org/doc/html/latest/admin-guide/pm/cpufreq.html?highlight=schedutil#policy-interface-in-sysfs>
use sysfs::Result;
use sysfs_macros::sysfs_attrs;

pub fn num_cpus() -> Result<usize> {
    std::fs::read_dir("/sys/devices/system/cpu/cpufreq")?.try_fold(0, |acc, res| match (acc, res) {
        (acc, Ok(inode))
            if {
                let name = inode.file_name();
                let name = name.to_string_lossy();
                name.starts_with("policy")
                    && name["policy".len()..].chars().all(|ch| ch.is_ascii_digit())
            } =>
        {
            Ok(acc + 1)
        }
        (acc, Ok(_)) => Ok(acc),
        (_, Err(e)) => Err(e.into()),
    })
}

#[sysfs_attrs(in "/sys/devices/system/cpu/cpufreq/policy{cpu}")]
pub mod cpufreq {
    use sysfs_macros::sysfs;

    /// List of online CPUs belonging to this policy (i.e. sharing the
    /// hardware performance scaling interface represented by the policyX
    /// policy object).
    #[sysfs]
    pub fn affected_cpus(cpu: usize) -> Vec<usize> {
        let read = |text: &str| text.split(' ').map(|int| int.parse().unwrap()).collect();
        ..
    }

    /// If the platform firmware (BIOS) tells the OS to apply an upper limit
    /// to CPU frequencies, that limit will be reported through this
    /// attribute (if present).
    ///
    /// The existence of the limit may be a result of some (often
    /// unintentional) BIOS settings, restrictions coming from a service
    /// processor or another BIOS/HW-based mechanisms.
    ///
    /// This does not cover ACPI thermal limitations which can be discovered
    /// through a generic thermal driver.
    ///
    /// This attribute is not present if the scaling driver in use does not
    /// support it.
    #[sysfs]
    pub fn bios_limit(cpu: usize) -> usize {
        let read = |text: &str| text.parse().unwrap();
        ..
    }

    /// Current frequency of the CPUs belonging to this policy as obtained
    /// from the hardware (in KHz).
    ///
    /// This is expected to be the frequency the hardware actually runs at.
    /// If that frequency cannot be determined, this attribute should not be
    /// present.
    #[sysfs]
    pub fn cpuinfo_cur_freq(cpu: usize) -> usize {
        let read = |text: &str| text.parse().unwrap();
        ..
    }

    /// Maximum possible operating frequency the CPUs belonging to this
    /// policy can run at (in kHz).
    #[sysfs]
    pub fn cpuinfo_max_freq(cpu: usize) -> usize {
        let read = |text: &str| text.parse().unwrap();
        ..
    }

    /// Minimum possible operating frequency the CPUs belonging to this
    /// policy can run at (in kHz).
    #[sysfs]
    pub fn cpuinfo_min_freq(cpu: usize) -> usize {
        let read = |text: &str| text.parse().unwrap();
        ..
    }

    /// The time it takes to switch the CPUs belonging to this policy from
    /// one P-state to another, in nanoseconds.
    ///
    /// If unknown or if known to be so high that the scaling driver does
    /// not work with the ondemand governor, -1 (CPUFREQ_ETERNAL) will be
    /// returned by reads from this attribute.
    #[sysfs]
    pub fn cpuinfo_transition_latency(cpu: usize) -> isize {
        let read = |text: &str| text.parse().unwrap();
        ..
    }

    /// List of all (online and offline) CPUs belonging to this policy.
    #[sysfs]
    pub fn related_cpus(cpu: usize) -> Vec<usize> {
        let read = |text: &str| text.split(' ').map(|int| int.parse().unwrap()).collect();
        ..
    }

    /// List of CPUFreq scaling governors present in the kernel that can be
    /// attached to this policy or (if the intel_pstate scaling driver is in
    /// use) list of scaling algorithms provided by the driver that can be
    /// applied to this policy.
    ///
    /// [Note that some governors are modular and it may be necessary to
    /// load a kernel module for the governor held by it to become available
    /// and be listed by this attribute.]
    #[sysfs]
    pub fn scaling_available_governors(cpu: usize) -> Vec<String> {
        let read = |text: &str| text.split(' ').map(ToOwned::to_owned).collect();
        ..
    }

    /// Current frequency of all of the CPUs belonging to this policy
    /// (in kHz).
    ///
    /// In the majority of cases, this is the frequency of the last P-state
    /// requested by the scaling driver from the hardware using the scaling
    /// interface provided by it, which may or may not reflect the frequency
    /// the CPU is actually running at (due to hardware design and other
    /// limitations).
    ///
    /// Some architectures (e.g. x86) may attempt to provide information
    /// more precisely reflecting the current CPU frequency through this
    /// attribute, but that still may not be the exact current CPU frequency
    /// as seen by the hardware at the moment.
    #[sysfs]
    pub fn scaling_cur_freq(cpu: usize) -> usize {
        let read = |text: &str| text.parse().unwrap();
        ..
    }

    /// The scaling driver currently in use.
    #[sysfs]
    pub fn scaling_driver(cpu: usize) -> String {
        let read = ToOwned::to_owned;
        ..
    }

    /// The scaling governor currently attached to this policy or (if the
    /// intel_pstate scaling driver is in use) the scaling algorithm
    /// provided by the driver that is currently applied to this policy.
    ///
    /// This attribute is read-write and writing to it will cause a new
    /// scaling governor to be attached to this policy or a new scaling
    /// algorithm provided by the scaling driver to be applied to it (in the
    /// intel_pstate case), as indicated by the string written to this
    /// attribute (which must be one of the names listed by the
    /// scaling_available_governors attribute described above).
    #[sysfs]
    pub fn scaling_governor(cpu: usize) -> String {
        let read = ToOwned::to_owned;
        let write = |gov: &str| gov.to_owned();
        ..
    }

    /// Maximum frequency the CPUs belonging to this policy are allowed to
    /// be running at (in kHz).
    ///
    /// This attribute is read-write and writing a string representing an
    /// integer to it will cause a new limit to be set (it must not be lower
    /// than the value of the scaling_min_freq attribute).
    #[sysfs]
    pub fn scaling_max_freq(cpu: usize) -> usize {
        let read = |text: &str| text.parse().unwrap();
        let write = |freq: usize| format!("{freq}");
        ..
    }

    /// Minimum frequency the CPUs belonging to this policy are allowed to
    /// be running at (in kHz).
    ///
    /// This attribute is read-write and writing a string representing a
    /// non-negative integer to it will cause a new limit to be set (it must
    /// not be higher than the value of the scaling_max_freq attribute).
    #[sysfs]
    pub fn scaling_min_freq(cpu: usize) -> usize {
        let read = |text: &str| text.parse().unwrap();
        let write = |freq: usize| format!("{freq}");
        ..
    }

    /// This attribute is functional only if the userspace scaling governor
    /// is attached to the given policy.
    ///
    /// It returns the last frequency requested by the governor (in kHz) or
    /// can be written to in order to set a new frequency for the policy.
    #[sysfs]
    pub fn scaling_setspeed(cpu: usize) -> usize {
        let read = |text: &str| text.parse().unwrap();
        let write = |freq: usize| format!("{freq}");
        ..
    }
}
