// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use crate::monitor::DriveMonitor;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::Length;
use cosmic::iced::futures::SinkExt;
use cosmic::iced::{Limits, Subscription, stream, window::Id};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::{column, row};
use cosmic_ext_applet_drives::{AppletMount, AppletMountType, get_all_devices, run_command};
use gio::VolumeMonitor;
use std::any::TypeId;
use std::future::pending;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct EventReceiver(Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<()>>>);

impl Hash for EventReceiver {
    fn hash<H: Hasher>(&self, state: &mut H) {
        TypeId::of::<VolumeMonitor>().hash(state);
    }
}

pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    appletmounts: Vec<AppletMount>,
    monitor: DriveMonitor,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    UpdateConfig(Config),
    Unmount(usize),
    Open(Option<String>),
    RefreshMounts,
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "dev.cappsy.CosmicExtAppletDrives";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let app = AppModel {
            core,
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            appletmounts: get_all_devices().unwrap_or_default(),
            monitor: DriveMonitor::new(),
            popup: None,
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("media-eject-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let mut content_list = widget::column(vec![]).padding(8).spacing(0);
        if self.appletmounts.is_empty() {
            content_list = content_list.push(row!(
                column!(
                    widget::button::text(fl!("no-devices-mounted")).on_press(Message::Open(None))
                )
                .width(Length::Fill)
            ));
        } else {
            for entry in &self.appletmounts {
                content_list = content_list.push(row!(
                    column!(widget::icon::from_name(match entry.device_type() {
                        AppletMountType::USB => "drive-harddisk-usb-symbolic",
                        AppletMountType::Network => "network-workgroup-symbolic",
                    }))
                    .padding([7, 5]),
                    column!(
                        widget::button::text(entry.label())
                            .on_press(Message::Open(entry.path()))
                            .width(Length::Fill)
                            .padding(5),
                    )
                    .width(Length::Fill),
                    column!(
                        widget::button::icon(widget::icon::from_name("media-eject-symbolic"))
                            .on_press(Message::Unmount(entry.idx()))
                    )
                ));
            }
        }

        self.core.applet.popup_container(content_list).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let event_rx = EventReceiver(self.monitor.event_rx.clone());
        Subscription::batch(vec![
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
            Subscription::run_with(event_rx, |event_rx| {
                let event_rx = event_rx.clone();
                stream::channel(
                    1,
                    |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
                        while let Some(()) = event_rx.0.lock().await.recv().await {
                            let _ = output.send(Message::RefreshMounts).await;
                        }
                        pending::<()>().await;
                        unreachable!()
                    },
                )
            }),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::Unmount(idx) => {
                self.monitor.unmount(idx);
            }
            Message::Open(mountpoint) => {
                run_command("cosmic-files", &mountpoint.unwrap_or_default());
            }
            Message::RefreshMounts => {
                self.appletmounts = get_all_devices().unwrap_or_default();
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    cosmic::surface::surface_task(cosmic::surface::action::destroy_popup(p))
                } else {
                    cosmic::surface::surface_task(cosmic::surface::action::app_popup(
                        |_| Default::default(),
                        |app: &mut AppModel| {
                            let new_id = Id::unique();
                            app.popup.replace(new_id);
                            let mut popup_settings = app.core.applet.get_popup_settings(
                                app.core.main_window_id().unwrap(),
                                new_id,
                                None,
                                None,
                                None,
                            );
                            popup_settings.positioner.size_limits = Limits::NONE
                                .max_width(372.0)
                                .min_width(300.0)
                                .min_height(200.0)
                                .max_height(1080.0);
                            popup_settings
                        },
                        None,
                    ))
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
