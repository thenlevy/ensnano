use crate::consts::*;
use iced_wgpu::wgpu;
use iced_winit::winit::dpi::{PhysicalPosition, PhysicalSize, Pixel};
use std::sync::{Arc, Mutex};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub mod bindgroup_manager;
pub mod camera2d;
pub mod chars2d;
pub mod circles2d;
pub mod instance;
pub mod light;
pub mod mesh;
pub mod texture;

pub fn create_buffer_with_data(
    device: &wgpu::Device,
    data: &[u8],
    usage: wgpu::BufferUsage,
) -> wgpu::Buffer {
    let descriptor = BufferInitDescriptor {
        label: Some("descriptor"),
        contents: data,
        usage,
    };
    device.create_buffer_init(&descriptor)
}

pub struct BufferDimensions {
    pub width: usize,
    pub height: usize,
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,
}

impl BufferDimensions {
    pub fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = std::mem::size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}

pub fn phantom_helix_encoder_nucl(
    design_id: u32,
    helix_id: u32,
    position: i32,
    forward: bool,
) -> u32 {
    let pos_id = (position + PHANTOM_RANGE) as u32 * 4 + if forward { 0 } else { 1 };
    let max_pos_id = (2 * PHANTOM_RANGE) as u32 * 4 + 3;
    let helix = helix_id * max_pos_id;
    assert!(helix <= 0xFF_FF_FF);
    (helix + pos_id) | (design_id << 24)
}

pub fn phantom_helix_encoder_bound(
    design_id: u32,
    helix_id: u32,
    position: i32,
    forward: bool,
) -> u32 {
    let pos_id = (position + PHANTOM_RANGE) as u32 * 4 + if forward { 2 } else { 3 };
    let max_pos_id = (2 * PHANTOM_RANGE) as u32 * 4 + 3;
    let helix = helix_id * max_pos_id;
    assert!(helix <= 0xFF_FF_FF);
    (helix + pos_id) | (design_id << 24)
}

pub fn phantom_helix_decoder(id: u32) -> PhantomElement {
    let max_pos_id = (2 * PHANTOM_RANGE) as u32 * 4 + 3;
    let design_id = id >> 24;
    let reminder = id & 0xFF_FF_FF;
    let helix_id = reminder / max_pos_id;
    let reminder = reminder % max_pos_id;
    let bound = reminder & 0b10 > 0;
    let forward = reminder % 2 == 0;
    let nucl_id = reminder / 4;
    let position = nucl_id as i32 - PHANTOM_RANGE;
    PhantomElement {
        design_id,
        helix_id,
        position,
        bound,
        forward,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhantomElement {
    pub design_id: u32,
    pub helix_id: u32,
    pub position: i32,
    pub bound: bool,
    pub forward: bool,
}

impl PhantomElement {
    pub fn to_nucl(&self) -> crate::design::Nucl {
        crate::design::Nucl {
            helix: self.helix_id as usize,
            position: self.position as isize,
            forward: self.forward,
        }
    }
}

pub fn message(message: Cow<'static, str>, level: rfd::MessageLevel) {
    let msg = rfd::AsyncMessageDialog::new()
        .set_level(level)
        .set_description(message.as_ref())
        .show();
    std::thread::spawn(move || futures::executor::block_on(msg));
}

pub fn new_color(color_idx: &mut usize) -> u32 {
    let color = {
        let hue = (*color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
        let saturation = (*color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.4;
        let value = (*color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.1;
        let hsv = color_space::Hsv::new(hue, saturation, value);
        let rgb = color_space::Rgb::from(hsv);
        (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
    };
    *color_idx += 1;
    color
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ndc {
    pub x: f32,
    pub y: f32,
}

unsafe impl bytemuck::Zeroable for Ndc {}
unsafe impl bytemuck::Pod for Ndc {}

impl Ndc {
    pub fn from_physical<S: Pixel, T: Pixel>(
        position: PhysicalPosition<S>,
        window_size: PhysicalSize<T>,
    ) -> Self {
        let position = position.cast::<f32>();
        let size = window_size.cast::<f32>();
        Self {
            x: position.x / size.width * 2. - 1.,
            y: position.y / size.height * -2. + 1.,
        }
    }
}

use crate::gui::{KeepProceed, Requests};
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
