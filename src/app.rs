// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{Limits, Subscription, window::Id};
use cosmic::iced_widget::row;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic_ext_applet_drives::{get_all_devices, run_command};

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    UpdateConfig(Config),
    Unmount(String),
    Open(String),
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
            ..Default::default()
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
        // Build applet view
        let devices = get_all_devices().unwrap_or_default();
        let mut content_list = widget::column().padding(8).spacing(0);
        if devices.is_empty() {
            content_list = content_list.push(row!(
                widget::button::text(fl!("no-devices-mounted"))
                    .on_press(Message::Open(String::new())),
            ));
        } else {
            for device in devices {
                content_list = content_list.push(row!(
                    widget::button::text(device.label())
                        .on_press(Message::Open(device.mountpoint())),
                    widget::button::icon(widget::icon::from_name("media-eject-symbolic"))
                        .on_press(Message::Unmount(device.mountpoint())),
                ));
            }
        }

        self.core.applet.popup_container(content_list).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::Unmount(mountpoint) => {
                // TODO: Unmounting is at its most basic right now.
                // This could be expanded to be more robust
                run_command("umount", &mountpoint);
            }
            Message::Open(mountpoint) => {
                // TODO: Launch the default file browser
                run_command("cosmic-files", &mountpoint);
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
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
                    get_popup(popup_settings)
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

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
