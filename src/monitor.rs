// SPDX-License-Identifier: GPL-3.0-only

use gio::{glib, prelude::*};
use std::future::pending;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

enum Cmd {
    Unmount(usize),
}

pub struct DriveMonitor {
    pub event_rx: Arc<Mutex<mpsc::UnboundedReceiver<()>>>,
    cmd_tx: mpsc::UnboundedSender<Cmd>,
}

impl DriveMonitor {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<()>();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<Cmd>();

        std::thread::spawn(move || {
            let main_loop = glib::MainLoop::new(None, false);
            main_loop.context().spawn_local(async move {
                let monitor = gio::VolumeMonitor::get();
                {
                    let event_tx = event_tx.clone();
                    monitor.connect_mount_added(move |_, _| {
                        let _ = event_tx.send(());
                    });
                }
                {
                    let event_tx = event_tx.clone();
                    monitor.connect_mount_removed(move |_, _| {
                        let _ = event_tx.send(());
                    });
                }

                while let Some(cmd) = cmd_rx.recv().await {
                    match cmd {
                        Cmd::Unmount(idx) => {
                            let mounts = monitor.mounts();
                            let non_shadowed: Vec<_> =
                                mounts.iter().filter(|m| !m.is_shadowed()).collect();
                            if let Some(mount) = non_shadowed.get(idx) {
                                if mount.can_eject() {
                                    mount.eject_with_operation(
                                        gio::MountUnmountFlags::NONE,
                                        gio::MountOperation::NONE,
                                        gio::Cancellable::NONE,
                                        |_| {},
                                    );
                                } else if mount.can_unmount() {
                                    mount.unmount_with_operation(
                                        gio::MountUnmountFlags::NONE,
                                        gio::MountOperation::NONE,
                                        gio::Cancellable::NONE,
                                        |_| {},
                                    );
                                }
                            }
                        }
                    }
                }

                pending::<()>().await;
            });
            main_loop.run();
        });

        Self {
            event_rx: Arc::new(Mutex::new(event_rx)),
            cmd_tx,
        }
    }

    pub fn unmount(&self, idx: usize) {
        let _ = self.cmd_tx.send(Cmd::Unmount(idx));
    }
}
