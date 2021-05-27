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

use super::{KeepProceed, Requests};
use std::sync::{Arc, Mutex};

use std::borrow::Cow;
pub fn yes_no_dialog(
    message: Cow<'static, str>,
    request: Arc<Mutex<Requests>>,
    yes: KeepProceed,
    no: Option<KeepProceed>,
) {
    let msg = rfd::AsyncMessageDialog::new()
        .set_description(message.as_ref())
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();
    std::thread::spawn(move || {
        let choice = async move {
            println!("thread spawned");
            let ret = msg.await;
            println!("about to send");
            if ret {
                request.lock().unwrap().keep_proceed = Some(yes);
            } else {
                request.lock().unwrap().keep_proceed = no;
            }
        };
        futures::executor::block_on(choice);
    });
}

pub fn message(message: Cow<'static, str>, level: rfd::MessageLevel) {
    let msg = rfd::AsyncMessageDialog::new()
        .set_level(level)
        .set_description(message.as_ref())
        .show();
    std::thread::spawn(move || futures::executor::block_on(msg));
}

pub fn blocking_message(
    message: Cow<'static, str>,
    level: rfd::MessageLevel,
    request: Arc<Mutex<Requests>>,
    keep_proceed: KeepProceed,
) {
    let msg = rfd::AsyncMessageDialog::new()
        .set_level(level)
        .set_description(message.as_ref())
        .show();
    std::thread::spawn(move || {
        futures::executor::block_on(msg);
        request.lock().unwrap().keep_proceed = Some(keep_proceed);
    });
}

pub fn save_before_new(requests: Arc<Mutex<Requests>>) {
    yes_no_dialog(
        "Do you want to save your design before loading an empty one?".into(),
        requests,
        KeepProceed::SaveBeforeNew,
        Some(KeepProceed::NewDesign),
    );
}

pub fn save_before_open(requests: Arc<Mutex<Requests>>) {
    yes_no_dialog(
        "Do you want to save your design before loading a new one?".into(),
        requests,
        KeepProceed::SaveBeforeOpen,
        Some(KeepProceed::LoadDesign),
    );
}

pub fn save_before_quit(requests: Arc<Mutex<Requests>>) {
    yes_no_dialog(
        "Do you want to save your design before exiting the app?".into(),
        requests,
        KeepProceed::SaveBeforeQuit,
        Some(KeepProceed::Quit),
    );
}
