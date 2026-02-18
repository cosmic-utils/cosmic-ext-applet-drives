use gio::prelude::*;
use notify_rust::Notification;
use std::fs;
use std::process::Command;

#[derive(Clone, Debug)]
pub enum AppletMountType {
    USB,
    Network,
}

#[derive(Clone, Debug)]
pub struct AppletMount {
    pub mount: gio::Mount,
    pub mount_type: AppletMountType,
    pub label: String,
    pub path: String,
}
impl AppletMount {
    #[must_use]
    pub fn device_type(&self) -> AppletMountType {
        self.mount_type.clone()
    }
    #[must_use]
    pub fn label(&self) -> String {
        self.label.clone()
    }
    #[must_use]
    pub fn path(&self) -> String {
        self.path.clone()
    }
}

pub fn get_all_devices() -> std::io::Result<Vec<AppletMount>> {
    let mut allmounts = vec![];

    let monitor = gio::VolumeMonitor::get();
    let mounts = monitor.mounts();
    let non_shadowed: Vec<_> = mounts.iter().filter(|m| !m.is_shadowed()).collect();

    for mount in non_shadowed {
        // Check if is a removable USB drive
        let is_removable = match mount.drive() {
            Some(drive) => drive.is_removable(),
            None => false,
        };

        // Check for remote drive
        let root = MountExt::root(mount);
        let is_remote = root
            .query_filesystem_info(
                gio::FILE_ATTRIBUTE_FILESYSTEM_REMOTE,
                gio::Cancellable::NONE,
            )
            .ok()
            .map(|info| info.boolean(gio::FILE_ATTRIBUTE_FILESYSTEM_REMOTE))
            .unwrap_or(true);

        //if is_removable || is_remote {
        allmounts.push(AppletMount {
            mount: mount.clone(),
            mount_type: if is_remote {
                AppletMountType::Network
            } else {
                AppletMountType::USB
            },
            label: mount.name().into(),
            path: mount.root().uri().into(),
        });
        //}
    }

    Ok(allmounts)
}

/*
pub fn get_all_devices() -> std::io::Result<Vec<AppletMount>> {
    let mut devices = vec![];

    // Removable / unmountable drives from /proc/mounts
    let mounts = procfs::mounts().unwrap();
    for mount in mounts {
        let mount_point = mount.fs_file.replace("\\040", " ");
        let mount_block = mount.fs_spec;

        if is_removable(&mount_block, &mount_point) {
            let device_info = device_info(&mount_block);
            devices.push(AppletMount {
                mount_type: if device_info.bus == Some(String::from("usb")) {
                    AppletMountType::USB
                } else {
                    AppletMountType::Disk
                },
                label: match device_info.label {
                    Some(label) => label,
                    None => {
                        // break up mountpoint to get fallback device label
                        let mountpoint_parts: Vec<&str> = mount_point.split('/').collect();
                        mountpoint_parts[mountpoint_parts.len() - 1].to_owned()
                    }
                },
                path: mount_point.clone(),
            });
        }
    }
    Ok(devices)
}
*/

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
