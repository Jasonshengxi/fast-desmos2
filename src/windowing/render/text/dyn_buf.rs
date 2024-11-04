use crate::windowing::render::util::Devices;
use bytemuck::NoUninit;
use std::marker::PhantomData;
use std::mem;
use wgpu::*;

fn create_bind_group<'a>(
    devices: &'a Devices,
    layout: &'a BindGroupLayout,
    buffer: &'a Buffer,
) -> BindGroup {
    devices.make_uniform_bind_group("instance bind group", layout, &[buffer])
}

pub fn create_bind_group_layout(devices: &Devices) -> BindGroupLayout {
    devices
        .device
        .create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("instance storage bind group"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    min_binding_size: None,
                    has_dynamic_offset: false,
                },
                count: None,
            }],
        })
}

fn create_buffer(devices: &Devices, size: BufferAddress, mapped_at_creation: bool) -> Buffer {
    devices.device.create_buffer(&BufferDescriptor {
        label: Some("instance buffer"),
        size,
        usage: BufferUsages::union(BufferUsages::COPY_DST, BufferUsages::STORAGE),
        mapped_at_creation,
    })
}

pub struct DynamicStorage<I: NoUninit> {
    length: u32,
    item_capacity: BufferAddress,

    buffer: Buffer,
    layout: BindGroupLayout,
    bind_group: BindGroup,

    _phantom: PhantomData<I>,
}

impl<I: NoUninit> DynamicStorage<I> {
    pub fn new(devices: &Devices) -> Self {
        Self::with_capacity(devices, 4)
    }

    pub fn len(&self) -> u32 {
        self.length
    }
    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }
    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn with_capacity(devices: &Devices, item_capacity: BufferAddress) -> Self {
        let byte_capacity = Self::item_to_byte_capacity(item_capacity);
        let buffer = create_buffer(devices, byte_capacity, false);
        let bind_group_layout = create_bind_group_layout(devices);
        let bind_group = create_bind_group(devices, &bind_group_layout, &buffer);

        Self {
            length: 0,
            item_capacity,
            buffer,
            layout: bind_group_layout,
            bind_group,
            _phantom: PhantomData,
        }
    }

    const fn item_to_byte_capacity(item_capacity: BufferAddress) -> BufferAddress {
        item_capacity * (size_of::<I>() as BufferAddress)
    }

    // pub fn shrink_to_fit(&mut self, devices: &Devices, command_encoder: &mut CommandEncoder) {
    //     let item_capacity = self.length as BufferAddress;
    //     let old_buffer = self.replace_buffer_with_new_length(devices, item_capacity, false);
    //
    //     command_encoder.copy_buffer_to_buffer(
    //         &old_buffer,
    //         0,
    //         &self.buffer,
    //         0,
    //         Self::item_to_byte_capacity(item_capacity),
    //     );
    // }

    pub fn set_new_data(&mut self, devices: &Devices, data: &[I]) {
        if data.len() <= self.item_capacity as usize {
            devices
                .queue
                .write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
        } else {
            let new_shape_capacity = (data.len() as BufferAddress).next_power_of_two();
            let new_data = bytemuck::cast_slice(data);
            self.replace_buffer_with_new_length(devices, new_shape_capacity, true);

            self.buffer.slice(..).get_mapped_range_mut()[..new_data.len()]
                .copy_from_slice(new_data);
            self.buffer.unmap();
        }
        self.length = data.len() as u32;
    }

    fn replace_buffer_with_new_length(
        &mut self,
        devices: &Devices,
        new_item_capacity: BufferAddress,
        mapped_at_creation: bool,
    ) -> Buffer {
        let new_byte_capacity = Self::item_to_byte_capacity(new_item_capacity);

        let new_buffer = create_buffer(devices, new_byte_capacity, mapped_at_creation);
        let new_bind_group = create_bind_group(devices, &self.layout, &new_buffer);

        let old_buffer = mem::replace(&mut self.buffer, new_buffer);
        self.bind_group = new_bind_group;
        self.item_capacity = new_item_capacity;

        old_buffer
    }
}
