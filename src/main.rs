use crate::rendering::{GpuCamera, GpuQuad, RenderData, RenderState};
use eframe::{egui, wgpu};

pub mod rendering;

struct App {
    last_time: Option<std::time::Instant>,
    info_window_open: bool,
    camera_window_open: bool,
    camera: Camera,
}

struct Camera {
    position: cgmath::Vector2<f32>,
    view_height: f32,
    move_speed: f32,
    zoom_speed: f32,
    show_grid: bool,
    grid_thickness: f32,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let renderer = cc.wgpu_render_state.as_ref().unwrap();
        let state = RenderState::new(renderer.target_format, &renderer.device, &renderer.queue);
        renderer.renderer.write().callback_resources.insert(state);

        Self {
            last_time: None,
            info_window_open: true,
            camera_window_open: true,
            camera: Camera {
                position: cgmath::Vector2 { x: 0.0, y: 0.0 },
                view_height: 10.0,
                move_speed: 1.0,
                zoom_speed: 2.0,
                show_grid: true,
                grid_thickness: 0.05,
            },
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let time = std::time::Instant::now();
        let dt = (time - self.last_time.unwrap_or(time)).as_secs_f32();
        self.last_time = Some(time);

        let mut quads = vec![];
        let circles = vec![];

        egui::TopBottomPanel::top("Menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.info_window_open |= ui.button("Info").clicked();
                self.camera_window_open |= ui.button("Camera").clicked();
            });
        });

        egui::Window::new("Info")
            .open(&mut self.info_window_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("FPS: {:.3}", 1.0 / dt));
                ui.label(format!("Frame Time: {:.3}ms", 1000.0 * dt));
            });

        egui::Window::new("Camera")
            .open(&mut self.camera_window_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.add(
                        egui::DragValue::new(&mut self.camera.position.x)
                            .speed(0.1)
                            .prefix("x:"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.camera.position.y)
                            .speed(0.1)
                            .prefix("y:"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("View Height:");
                    ui.add(egui::DragValue::new(&mut self.camera.view_height).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Move Speed:");
                    ui.add(egui::DragValue::new(&mut self.camera.move_speed).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Zoom Speed:");
                    ui.add(egui::DragValue::new(&mut self.camera.zoom_speed).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Show Grid:");
                    ui.checkbox(&mut self.camera.show_grid, "");
                });
                ui.horizontal(|ui| {
                    ui.label("Grid Thickness:");
                    ui.add(egui::DragValue::new(&mut self.camera.grid_thickness).speed(0.01));
                });
            });

        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                self.camera.position.y += i.key_down(egui::Key::W) as u8 as f32
                    * (self.camera.move_speed * self.camera.view_height * dt);
                self.camera.position.y -= i.key_down(egui::Key::S) as u8 as f32
                    * (self.camera.move_speed * self.camera.view_height * dt);
                self.camera.position.x -= i.key_down(egui::Key::A) as u8 as f32
                    * (self.camera.move_speed * self.camera.view_height * dt);
                self.camera.position.x += i.key_down(egui::Key::D) as u8 as f32
                    * (self.camera.move_speed * self.camera.view_height * dt);

                self.camera.view_height += i.key_down(egui::Key::Q) as u8 as f32
                    * (self.camera.zoom_speed * self.camera.view_height * dt);
                self.camera.view_height -= i.key_down(egui::Key::E) as u8 as f32
                    * (self.camera.zoom_speed * self.camera.view_height * dt);
            });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(50, 50, 50)))
            .show(ctx, |ui| {
                let (rect, _response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
                let aspect = rect.width() / rect.height();

                if self.camera.show_grid {
                    let view_width = self.camera.view_height * aspect;
                    for i in 0..view_width.ceil() as usize + 2 {
                        let position =
                            (i as f32 - view_width * 0.5 - 1.0 + self.camera.position.x).round();
                        quads.push(GpuQuad {
                            position: cgmath::Vector3 {
                                x: position,
                                y: self.camera.position.y,
                                z: if position == 0.0 { 0.1 } else { 0.0 },
                            },
                            rotation: 0.0,
                            color: if position == 0.0 {
                                cgmath::Vector3 {
                                    x: 1.0,
                                    y: 1.0,
                                    z: 1.0,
                                }
                            } else {
                                cgmath::Vector3 {
                                    x: 0.5,
                                    y: 0.5,
                                    z: 0.5,
                                }
                            },
                            size: cgmath::Vector2 {
                                x: self.camera.grid_thickness,
                                y: self.camera.view_height,
                            },
                        });
                    }
                    for i in 0..self.camera.view_height.ceil() as usize + 2 {
                        let position = (i as f32 - self.camera.view_height * 0.5 - 1.0
                            + self.camera.position.y)
                            .round();
                        quads.push(GpuQuad {
                            position: cgmath::Vector3 {
                                x: self.camera.position.x,
                                y: position,
                                z: if position == 0.0 { 0.1 } else { 0.0 },
                            },
                            rotation: 0.0,
                            color: if position == 0.0 {
                                cgmath::Vector3 {
                                    x: 1.0,
                                    y: 1.0,
                                    z: 1.0,
                                }
                            } else {
                                cgmath::Vector3 {
                                    x: 0.5,
                                    y: 0.5,
                                    z: 0.5,
                                }
                            },
                            size: cgmath::Vector2 {
                                x: view_width,
                                y: self.camera.grid_thickness,
                            },
                        });
                    }
                }

                self.camera.view_height = self.camera.view_height.max(0.1);
                ui.painter()
                    .add(eframe::egui_wgpu::Callback::new_paint_callback(
                        rect,
                        RenderData {
                            camera: GpuCamera {
                                position: self.camera.position,
                                vertical_height: self.camera.view_height,
                                aspect,
                            },
                            quads,
                            circles,
                        },
                    ));
            });

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Geometric Algebra Sandbox",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            vsync: false,
            depth_buffer: 24,
            wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
                present_mode: wgpu::PresentMode::AutoNoVsync,
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
