use gio::prelude::*;
use notify_rust::Notification;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug)]
pub enum AppletMountType {
    USB,
    Network,
}

#[derive(Clone, Debug)]
pub struct AppletMount {
    pub idx: usize,
    pub mount: gio::Mount,
    pub mount_type: AppletMountType,
    pub label: String,
    pub path: Option<PathBuf>,
}
impl AppletMount {
    #[must_use]
    pub fn idx(&self) -> usize {
        self.idx
    }
    #[must_use]
    pub fn device_type(&self) -> &AppletMountType {
        &self.mount_type
    }
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    #[must_use]
    pub fn path(&self) -> Option<String> {
        self.path.clone()?.as_path().to_str().map(|s| s.to_string())
    }
}

pub fn get_all_devices() -> std::io::Result<Vec<AppletMount>> {
    let mut allmounts = vec![];

    let monitor = gio::VolumeMonitor::get();
    let mounts = monitor.mounts();
    let non_shadowed: Vec<_> = mounts.iter().filter(|m| !m.is_shadowed()).collect();

    for (idx,mount) in non_shadowed.into_iter().enumerate() {
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

        // This keeps inetrnal hd mounts off the list
        if is_removable || is_remote {
            allmounts.push(AppletMount {
                idx,
                mount: mount.clone(),
                mount_type: if is_remote {
                    AppletMountType::Network
                } else {
                    AppletMountType::USB
                },
                label: mount.name().into(),
                path: mount.root().path(),
            });
        }
    }

    Ok(allmounts)
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
