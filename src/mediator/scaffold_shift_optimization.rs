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

use std::sync::mpsc;

impl Mediator {
    pub fn optimize_shift<R: ShiftOptimizerReader>(&mut self, d_id: usize, reader: &mut R) {
        let computing = self.computing.clone();
        let design = self.designs[d_id].clone();
        let messages = self.messages.clone();
        let (send_progress, rcv_progress) = mpsc::channel::<f32>();
        reader.attach_progress_chanel(rcv_progress);
        let (send_result, rcv_result) = mpsc::channel::<ShiftOptimizationResult>();
        reader.attach_result_chanel(rcv_result);
        std::thread::spawn(move || {
            let (position, score) = design.read().unwrap().optimize_shift(send_progress);
            send_result
                .send(ShiftOptimizationResult { position, score })
                .unwrap();
        });
        /*
        std::thread::spawn(move || {
            let (send, rcv) = std::sync::mpsc::channel::<f32>();
            std::thread::spawn(move || {
                *computing.lock().unwrap() = true;
                let (position, score) = design.read().unwrap().optimize_shift(send);
                let msg = format!("Scaffold position set to {}\n {}", position, score);
                message(msg.into(), rfd::MessageLevel::Info);
                *computing.lock().unwrap() = false;
            });
            while let Ok(progress) = rcv.recv() {
                messages
                    .lock()
                    .unwrap()
                    .push_progress("Optimizing position".to_string(), progress)
            }
            messages.lock().unwrap().finish_progess();
        });
        */
    }
}

pub trait ShiftOptimizerReader {
    fn attach_progress_chanel(&mut self, chanel: mpsc::Receiver<f32>);
    fn attach_result_chanel(&mut self, chanel: mpsc::Receiver<ShiftOptimizationResult>);
}

pub struct ShiftOptimizationResult {
    position: usize,
    score: usize,
}
