use crate::consts::*;
use iced_wgpu::wgpu;
use native_dialog::{MessageDialog, MessageType};
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

pub fn message(message: MessageDialog<'static>) {
    if cfg!(target_os = "windows") {
        std::thread::spawn(move || {
            message.show_alert().unwrap();
        });
    } else {
        message.show_alert().unwrap();
    }
}

pub fn yes_no(text: &'static str) -> bool {
    if cfg!(target_os = "macos") {
        MessageDialog::new()
            .set_type(MessageType::Info)
            .set_text(text)
            .show_confirm()
            .unwrap()
    } else {
        let (choice_snd, choice_rcv) = std::sync::mpsc::channel::<bool>();
        std::thread::spawn(move || {
            let choice = MessageDialog::new()
                .set_type(MessageType::Info)
                .set_text(text)
                .show_confirm()
                .unwrap();
            choice_snd.send(choice).unwrap();
        });
        choice_rcv.recv().unwrap()
    }
}
