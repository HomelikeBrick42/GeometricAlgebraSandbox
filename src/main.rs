use crate::{
    multivector::Multivector,
    rendering::{GpuCamera, GpuQuad, RenderData, RenderState},
};
use eframe::{egui, wgpu};

pub mod multivector;
pub mod rendering;

struct App {
    last_time: Option<std::time::Instant>,
    info_window_open: bool,
    camera_window_open: bool,
    camera: Camera,
    parameters_window_open: bool,
    parameters: Vec<Parameter>,
}

struct Camera {
    position: cgmath::Vector2<f32>,
    view_height: f32,
    move_speed: f32,
    zoom_speed: f32,
    show_grid: bool,
    grid_thickness: f32,
}

struct Parameter {
    name: String,
    type_: ParameterType,
    color: cgmath::Vector3<f32>,
    value: Multivector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParameterType {
    Grade0,
    Grade1,
    Grade2,
    Grade3,
    Multivector,
}

impl ParameterType {
    pub fn display_name(&self) -> &'static str {
        match *self {
            ParameterType::Grade0 => "Scalar",
            ParameterType::Grade1 => "Grade 1",
            ParameterType::Grade2 => "Grade 2",
            ParameterType::Grade3 => "Grade 3",
            ParameterType::Multivector => "Multivector",
        }
    }
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
            parameters_window_open: true,
            parameters: vec![],
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
                self.parameters_window_open |= ui.button("Parameters").clicked();
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

        egui::Window::new("Parameters")
            .open(&mut self.parameters_window_open)
            .scroll([false, true])
            .show(ctx, |ui| {
                if ui.button("New Parameter").clicked() {
                    self.parameters.push(Parameter {
                        name: "Unnamed Parameter".into(),
                        type_: ParameterType::Grade0,
                        color: cgmath::Vector3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        value: Multivector::ZERO,
                    });
                }
                let mut i = 0usize;
                let mut delete = false;
                self.parameters.retain_mut(|parameter| {
                    egui::CollapsingHeader::new(&parameter.name)
                        .id_salt(i)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut parameter.name);
                            });

                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                ui.color_edit_button_rgb(parameter.color.as_mut());
                            });

                            ui.horizontal(|ui| {
                                ui.label("Type:");
                                if egui::ComboBox::from_id_salt("type")
                                    .selected_text(parameter.type_.display_name())
                                    .show_ui(ui, |ui| {
                                        for type_ in [
                                            ParameterType::Grade0,
                                            ParameterType::Grade1,
                                            ParameterType::Grade2,
                                            ParameterType::Grade3,
                                            ParameterType::Multivector,
                                        ] {
                                            ui.selectable_value(
                                                &mut parameter.type_,
                                                type_,
                                                type_.display_name(),
                                            );
                                        }
                                    })
                                    .response
                                    .changed()
                                {
                                    parameter.value = match parameter.type_ {
                                        ParameterType::Grade0 => parameter.value.grade0(),
                                        ParameterType::Grade1 => parameter.value.grade1(),
                                        ParameterType::Grade2 => parameter.value.grade2(),
                                        ParameterType::Grade3 => parameter.value.grade3(),
                                        ParameterType::Multivector => parameter.value,
                                    };
                                }
                            });

                            let (grade0, grade1, grade2, grade3) = match parameter.type_ {
                                ParameterType::Grade0 => (true, false, false, false),
                                ParameterType::Grade1 => (false, true, false, false),
                                ParameterType::Grade2 => (false, false, true, false),
                                ParameterType::Grade3 => (false, false, false, true),
                                ParameterType::Multivector => (true, true, true, true),
                            };

                            if grade0 {
                                ui.horizontal(|ui| {
                                    ui.label("Scalar:");
                                    ui.add(egui::DragValue::new(&mut parameter.value.s).speed(0.1));
                                });
                            }
                            if grade1 {
                                ui.horizontal(|ui| {
                                    ui.label("e0:");
                                    ui.add(
                                        egui::DragValue::new(&mut parameter.value.e0).speed(0.1),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("e1:");
                                    ui.add(
                                        egui::DragValue::new(&mut parameter.value.e1).speed(0.1),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("e2:");
                                    ui.add(
                                        egui::DragValue::new(&mut parameter.value.e2).speed(0.1),
                                    );
                                });
                            }
                            if grade2 {
                                ui.horizontal(|ui| {
                                    ui.label("e01:");
                                    ui.add(
                                        egui::DragValue::new(&mut parameter.value.e01).speed(0.1),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("e02:");
                                    ui.add(
                                        egui::DragValue::new(&mut parameter.value.e02).speed(0.1),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("e12:");
                                    ui.add(
                                        egui::DragValue::new(&mut parameter.value.e12).speed(0.1),
                                    );
                                });
                            }
                            if grade3 {
                                ui.horizontal(|ui| {
                                    ui.label("e012:");
                                    ui.add(
                                        egui::DragValue::new(&mut parameter.value.e012).speed(0.1),
                                    );
                                });
                            }

                            delete = ui.button("Delete").clicked();
                        });

                    i += 1;
                    !delete
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
