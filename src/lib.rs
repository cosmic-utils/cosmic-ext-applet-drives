use notify_rust::Notification;
use std::fs;
use std::process::Command;

#[derive(Clone)]
pub enum DeviceType {
    USB,
    Disk,
    Network,
}

#[derive(Clone)]
pub struct Device {
    device_type: DeviceType,
    label: String,
    mountpoint: String,
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
}

pub fn get_all_devices() -> std::io::Result<Vec<Device>> {
    let mut devices = vec![];

    // Removable / unmountable drives from /proc/mounts
    let mounts = procfs::mounts().unwrap();
    for mount in mounts {
        let mount_point = mount.fs_file.replace("\\040", " ");
        let mount_block = mount.fs_spec;

        if is_removable(&mount_block, &mount_point) {
            let device_info = device_info(&mount_block);
            devices.push(Device {
                device_type: if device_info.bus == Some(String::from("usb")) {
                    DeviceType::USB
                } else {
                    DeviceType::Disk
                },
                label: match device_info.label {
                    Some(label) => label,
                    None => {
                        // break up mountpoint to get fallback device label
                        let mountpoint_parts: Vec<&str> = mount_point.split('/').collect();
                        mountpoint_parts[mountpoint_parts.len() - 1].to_owned()
                    }
                },
                mountpoint: mount_point.clone(),
            });
        }
    }
    Ok(devices)
}

// Get whatever extra information is useful from udev
#[derive(Debug)]
struct DeviceInfo {
    fs: Option<String>,
    bus: Option<String>,
    label: Option<String>,
}

fn device_info(mount_block: &str) -> DeviceInfo {
    udev::Enumerator::new()
        .and_then(|mut e| {
            let device_name = mount_block.strip_prefix("/dev/").unwrap_or(mount_block);
            e.match_sysname(device_name)?;
            let devices: Vec<_> = e.scan_devices()?.collect();
            Ok(devices)
        })
        .ok()
        .and_then(|devices| devices.into_iter().next())
        .map(|dev| DeviceInfo {
            fs: dev
                .property_value("ID_FS_TYPE")
                .map(|v| v.to_string_lossy().to_string()),
            bus: dev
                .property_value("ID_BUS")
                .map(|v| v.to_string_lossy().to_string()),
            label: dev
                .property_value("ID_FS_LABEL")
                .map(|v| v.to_string_lossy().to_string()),
        })
        .unwrap_or_else(|| DeviceInfo {
            fs: None,
            bus: None,
            label: None,
        })
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
