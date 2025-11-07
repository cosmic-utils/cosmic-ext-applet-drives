use notify_rust::Notification;
use std::fs;
use std::process::Command;

#[derive(Clone)]
pub enum DeviceType {
    USB,
    _Network,
}

#[derive(Clone)]
pub struct Device {
    device_type: DeviceType,
    label: String,
    mountpoint: String,
    mounted: bool,
}

impl Device {
    #[must_use]
    pub fn device_type(&self) -> DeviceType {
        self.device_type.clone()
    }
    #[must_use]
    pub fn label(&self) -> String {
        self.label.clone()
    }
    #[must_use]
    pub fn mountpoint(&self) -> String {
        self.mountpoint.clone()
    }
    #[must_use]
    pub fn mounted(&self) -> bool {
        self.mounted
    }
}

pub fn get_all_devices() -> std::io::Result<Vec<Device>> {
    let mut devices = vec![];

    // Removable / unmountable drives from /proc/mounts
    let mounts = procfs::mounts().unwrap();
    for mount in mounts {
        let mount_point = mount.fs_file.replace("\\040", " ");
        let mount_block = mount.fs_spec;

        if is_removable(&mount_block, &mount_point) {
            // break up mountpoint to get the device label
            let mountpoint_parts: Vec<&str> = mount_point.split('/').collect();
            let label = mountpoint_parts[mountpoint_parts.len() - 1];
            devices.push(Device {
                device_type: DeviceType::USB,
                label: get_partition_label(&mount_block).unwrap_or(label.to_owned()),
                mountpoint: mount_point.clone(),
                mounted: true,
            });
        }
    }
    Ok(devices)
}

fn is_removable(mount_block: &str, mount_point: &str) -> bool {
    // pass early if mounted somewhere we want to show
    // this helps with drives that aren't flagged as removable
    if mount_point.starts_with("/run/media/") || mount_point.starts_with("/media/") {
        return true;
    }

    // fallback on the removable flag
    fs::read_to_string(format!(
        "/sys/block/{}/removable",
        mount_block
            .replace("/dev/", "")
            .trim_end_matches(|c: char| c.is_ascii_digit())
    ))
    .map(|t| t.trim() == "1")
    .unwrap_or(false)
}

fn get_partition_label(mount_block: &str) -> Option<String> {
    let output = Command::new("blkid").output();

    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with(mount_block) {
                    if let Some(label_start) = line.find("LABEL=") {
                        let label_part = &line[label_start..];
                        if let Some(label_end) = label_part.find(' ') {
                            return Some(label_part[6..label_end - 1].to_owned());
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn run_command(cmd: &str, mountpoint: &str) {
    match if is_flatpak() {
        Command::new("flatpak-spawn")
            .arg("--host")
            .arg(cmd)
            .arg(mountpoint)
            .status()
    } else {
        Command::new(cmd).arg(mountpoint).status()
    } {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Error: {err}");
        }
    }
}

pub fn _send_notification(title: &str, desc: &str) {
    match Notification::new()
        .summary(title)
        .body(desc)
        .icon("media-eject-symbolic")
        .show()
    {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Error: {err}");
        }
    }
}

#[cfg(feature = "flatpak")]
fn is_flatpak() -> bool {
    true
}

#[cfg(not(feature = "flatpak"))]
fn is_flatpak() -> bool {
    false
}
