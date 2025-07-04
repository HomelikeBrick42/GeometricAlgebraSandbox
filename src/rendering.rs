use crate::multivector::Multivector;
use eframe::{egui, wgpu};
use encase::{ArrayLength, ShaderSize, ShaderType};

#[derive(ShaderType)]
pub struct GpuCamera {
    pub transform: Multivector,
    pub vertical_height: f32,
    pub aspect: f32,
    pub line_thickness: f32,
    pub point_radius: f32,
    pub flavour: u32,
}

#[derive(ShaderType)]
pub struct GpuObject {
    pub value: Multivector,
    pub color: cgmath::Vector3<f32>,
    pub layer: f32,
}

#[derive(ShaderType)]
struct GpuObjects<'a> {
    count: ArrayLength,
    #[size(runtime)]
    data: &'a Vec<GpuObject>,
}

pub struct RenderState {
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    objects_buffer: wgpu::Buffer,
    objects_bind_group_layout: wgpu::BindGroupLayout,
    objects_bind_group: wgpu::BindGroup,

    objects_render_pipeline: wgpu::RenderPipeline,
}

impl RenderState {
    pub fn new(
        target_format: wgpu::TextureFormat,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> Self {
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: GpuCamera::SHADER_SIZE.get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuCamera::SHADER_SIZE),
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let objects_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Objects Buffer"),
            size: GpuObjects::min_size().get(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let objects_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Objects Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuObjects::min_size()),
                    },
                    count: None,
                }],
            });
        let objects_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Objects Bind Group"),
            layout: &objects_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: objects_buffer.as_entire_binding(),
            }],
        });

        let objects_shader = device.create_shader_module(wgpu::include_wgsl!("./objects.wgsl"));

        let objects_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Objects Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &objects_bind_group_layout],
                push_constant_ranges: &[],
            });
        let objects_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Objects Render Pipeline"),
                layout: Some(&objects_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &objects_shader,
                    entry_point: Some("vertex"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 4,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &objects_shader,
                    entry_point: Some("fragment"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        Self {
            camera_buffer,
            camera_bind_group,

            objects_buffer,
            objects_bind_group_layout,
            objects_bind_group,

            objects_render_pipeline,
        }
    }
}

pub struct RenderData {
    pub camera: GpuCamera,
    pub objects: Vec<GpuObject>,
}

impl eframe::egui_wgpu::CallbackTrait for RenderData {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &eframe::egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let state: &mut RenderState = callback_resources.get_mut().unwrap();

        {
            let mut camera_buffer = queue
                .write_buffer_with(&state.camera_buffer, 0, GpuCamera::SHADER_SIZE)
                .unwrap();
            encase::UniformBuffer::new(&mut *camera_buffer)
                .write(&self.camera)
                .unwrap();
        }

        {
            let objects = GpuObjects {
                count: ArrayLength,
                data: &self.objects,
            };

            let size = objects.size();
            if size.get() > state.objects_buffer.size() {
                state.objects_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Objects Buffer"),
                    size: size.get(),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                state.objects_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Objects Bind Group"),
                    layout: &state.objects_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: state.objects_buffer.as_entire_binding(),
                    }],
                });
            }

            let mut objects_buffer = queue
                .write_buffer_with(&state.objects_buffer, 0, size)
                .unwrap();
            encase::StorageBuffer::new(&mut *objects_buffer)
                .write(&objects)
                .unwrap();
        }

        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &eframe::egui_wgpu::CallbackResources,
    ) {
        let state: &RenderState = callback_resources.get().unwrap();

        render_pass.set_pipeline(&state.objects_render_pipeline);
        render_pass.set_bind_group(0, &state.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &state.objects_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
}
