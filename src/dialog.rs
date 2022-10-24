/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

//! Functions that create [states](`crate::controller::State`) in which the user is interacting
//! with a dialog box (alert, file picker, ...).

macro_rules! log_err {
    ($x:expr) => {
        if $x.is_err() {
            log::error!("Unexpected error")
        }
    };
}

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
pub type Filters = &'static [(&'static str, &'static [&'static str])];

use std::borrow::Cow;
/// A question to which the user must answer yes or no
pub struct YesNoQuestion(mpsc::Receiver<bool>);
impl YesNoQuestion {
    pub fn answer(&self) -> Option<bool> {
        self.0.try_recv().ok()
    }
}

pub fn yes_no_dialog(message: Cow<'static, str>) -> YesNoQuestion {
    let msg = rfd::AsyncMessageDialog::new()
        .set_description(message.as_ref())
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();
    let (snd, rcv) = mpsc::channel();
    thread::spawn(move || {
        let choice = async move {
            println!("thread spawned");
            let ret = msg.await;
            println!("about to send");
            log_err![snd.send(ret)];
        };
        futures::executor::block_on(choice);
    });
    YesNoQuestion(rcv)
}

/// A message that the user must acknowledge
pub struct MustAckMessage(mpsc::Receiver<()>);
impl MustAckMessage {
    pub fn was_ack(&self) -> bool {
        self.0.try_recv().is_ok()
    }
}

pub fn blocking_message(message: Cow<'static, str>, level: rfd::MessageLevel) -> MustAckMessage {
    let msg = rfd::AsyncMessageDialog::new()
        .set_level(level)
        .set_description(message.as_ref())
        .show();
    let (snd, rcv) = mpsc::channel();
    thread::spawn(move || {
        futures::executor::block_on(msg);
        log_err!(snd.send(()))
    });
    MustAckMessage(rcv)
}

pub struct PathInput(mpsc::Receiver<Option<PathBuf>>);
impl PathInput {
    pub fn get(&self) -> Option<Option<PathBuf>> {
        self.0.try_recv().ok()
    }
}

fn filter_has_extension(filter: &Filters, extension: &str) -> bool {
    for f in filter.iter() {
        if f.1.contains(&extension) {
            return true;
        }
    }
    false
}

pub fn get_file_to_write<P1: AsRef<Path>, P2: AsRef<Path>>(
    extension_filter: &'static Filters,
    starting_path: Option<P1>,
    starting_name: Option<P2>,
) -> PathInput {
    log::info!(
        "starting path {:?}",
        starting_path.as_ref().and_then(|p| p.as_ref().to_str())
    );
    log::info!(
        "starting name {:?}",
        starting_name.as_ref().and_then(|p| p.as_ref().to_str())
    );
    let mut dialog = rfd::AsyncFileDialog::new();

    let default_extenstion = extension_filter.get(0).and_then(|f| f.1.get(0));

    let starting_name = starting_name.and_then(|p| {
        let mut path_buf = PathBuf::from(p.as_ref());
        let extension = path_buf.extension();
        if extension.is_none() && default_extenstion.is_some() {
            path_buf.set_extension(default_extenstion.unwrap());
        } else if let Some(_current_extension) = extension
            .filter(|ext| !filter_has_extension(extension_filter, ext.to_str().unwrap_or("")))
        {
            let new_extension = format!("{}", default_extenstion.unwrap_or(&""));
            path_buf.set_extension(new_extension);
        }
        path_buf.file_name().map(|s| s.to_os_string())
    });
    log::info!("starting name filtered {:?}", starting_name);
    for filter in extension_filter.iter() {
        dialog = dialog.add_filter(filter.0, filter.1);
    }
    log::info!(
        "starting path filtered {:?}",
        starting_path.as_ref().map(|p| p.as_ref())
    );
    if let Some(path) = starting_path {
        dialog = dialog.set_directory(path);
    }
    if let Some(name) = starting_name {
        dialog = dialog.set_file_name(&name.to_string_lossy());
    }
    let future_file = dialog.save_file();
    let (snd, rcv) = mpsc::channel();
    thread::spawn(move || {
        let save_op = async move {
            let file = future_file.await;
            if let Some(handle) = file {
                let mut path_buf: std::path::PathBuf = handle.path().into();
                let extension = path_buf.extension();
                if extension.is_none() && default_extenstion.is_some() {
                    path_buf.set_extension(default_extenstion.unwrap());
                } else if let Some(current_extension) = extension.filter(|ext| {
                    !filter_has_extension(extension_filter, ext.to_str().unwrap_or(""))
                }) {
                    let new_extension = format!(
                        "{}.{}",
                        current_extension.to_str().unwrap(),
                        default_extenstion.unwrap_or(&"")
                    );
                    path_buf.set_extension(new_extension);
                }
                log_err![snd.send(Some(path_buf))];
            } else {
                log_err![snd.send(None)];
            }
        };
        futures::executor::block_on(save_op);
    });
    PathInput(rcv)
}

#[allow(dead_code)]
pub fn get_dir() -> PathInput {
    let dialog = rfd::AsyncFileDialog::new().pick_folder();
    let (snd, rcv) = mpsc::channel();
    thread::spawn(move || {
        let save_op = async move {
            let file = dialog.await;
            if let Some(handle) = file {
                let path_buf: std::path::PathBuf = handle.path().into();
                log_err![snd.send(Some(path_buf))];
            } else {
                log_err![snd.send(None)];
            }
        };
        futures::executor::block_on(save_op);
    });
    PathInput(rcv)
}

pub fn load<P: AsRef<Path>>(starting_path: Option<P>, filters: Filters) -> PathInput {
    let mut dialog = rfd::AsyncFileDialog::new();
    for filter in filters.iter() {
        dialog = dialog.add_filter(filter.0, filter.1);
    }
    log::info!(
        "starting path {:?}",
        starting_path.as_ref().map(|p| p.as_ref())
    );
    if let Some(path) = starting_path {
        dialog = dialog.set_directory(path);
    }
    let future_file = dialog.pick_file();
    let (snd, rcv) = mpsc::channel();
    thread::spawn(move || {
        let load_op = async move {
            let file = future_file.await;
            if let Some(handle) = file {
                let path_buf: std::path::PathBuf = handle.path().into();
                log_err![snd.send(Some(path_buf))];
            } else {
                log_err![snd.send(None)];
            }
        };
        futures::executor::block_on(load_op);
    });
    PathInput(rcv)
}
