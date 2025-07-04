use crate::{
    evaluation::evaluate_expression,
    multivector::Multivector,
    parsing::{AstStatementKind, parse},
    rendering::{GpuCamera, GpuObject, RenderData, RenderState},
};
use eframe::{egui, wgpu};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub mod evaluation;
pub mod lexer;
pub mod multivector;
pub mod parsing;
pub mod rendering;

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct App {
    #[serde(skip)]
    last_time: Option<std::time::Instant>,
    info_window_open: bool,
    camera_window_open: bool,
    camera: Camera,
    parameters_window_open: bool,
    parameters: Vec<Parameter>,
    code_window_open: bool,
    errors: Vec<String>,
    code: String,
    variables_window_open: bool,
    variables: BTreeMap<String, Variable>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            last_time: None,
            info_window_open: true,
            camera_window_open: true,
            camera: Camera {
                transform: Multivector {
                    s: 1.0,
                    ..Multivector::ZERO
                },
                view_height: 10.0,
                move_speed: 1.0,
                zoom_speed: 2.0,
                line_thickness: 0.1,
                point_radius: 0.1,
            },
            parameters_window_open: true,
            parameters: vec![
                Parameter {
                    name: "e0".into(),
                    type_: ParameterType::Grade1,
                    value: Multivector {
                        e0: 1.0,
                        ..Multivector::ZERO
                    },
                },
                Parameter {
                    name: "e1".into(),
                    type_: ParameterType::Grade1,
                    value: Multivector {
                        e1: 1.0,
                        ..Multivector::ZERO
                    },
                },
                Parameter {
                    name: "e2".into(),
                    type_: ParameterType::Grade1,
                    value: Multivector {
                        e2: 1.0,
                        ..Multivector::ZERO
                    },
                },
                Parameter {
                    name: "e01".into(),
                    type_: ParameterType::Grade2,
                    value: Multivector {
                        e01: 1.0,
                        ..Multivector::ZERO
                    },
                },
                Parameter {
                    name: "e02".into(),
                    type_: ParameterType::Grade2,
                    value: Multivector {
                        e02: 1.0,
                        ..Multivector::ZERO
                    },
                },
                Parameter {
                    name: "e12".into(),
                    type_: ParameterType::Grade2,
                    value: Multivector {
                        e12: 1.0,
                        ..Multivector::ZERO
                    },
                },
                Parameter {
                    name: "e012".into(),
                    type_: ParameterType::Grade3,
                    value: Multivector {
                        e012: 1.0,
                        ..Multivector::ZERO
                    },
                },
            ],
            code_window_open: true,
            errors: vec![],
            code: String::new(),
            variables_window_open: true,
            variables: BTreeMap::from([
                (
                    "e1".into(),
                    Variable {
                        value: Multivector::ZERO,
                        display: Some(VariableDisplay {
                            color: cgmath::Vector3 {
                                x: 1.0,
                                y: 0.0,
                                z: 0.0,
                            },
                            layer: 0.0,
                        }),
                    },
                ),
                (
                    "e2".into(),
                    Variable {
                        value: Multivector::ZERO,
                        display: Some(VariableDisplay {
                            color: cgmath::Vector3 {
                                x: 0.0,
                                y: 1.0,
                                z: 0.0,
                            },
                            layer: 0.0,
                        }),
                    },
                ),
                (
                    "e12".into(),
                    Variable {
                        value: Multivector::ZERO,
                        display: Some(VariableDisplay {
                            color: cgmath::Vector3 {
                                x: 1.0,
                                y: 1.0,
                                z: 1.0,
                            },
                            layer: 0.01,
                        }),
                    },
                ),
            ]),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Variable {
    #[serde(default, skip)]
    pub value: Multivector,
    pub display: Option<VariableDisplay>,
}

#[derive(Serialize, Deserialize)]
pub struct VariableDisplay {
    pub color: cgmath::Vector3<f32>,
    pub layer: f32,
}

#[derive(Serialize, Deserialize)]
struct Camera {
    transform: Multivector,
    view_height: f32,
    move_speed: f32,
    zoom_speed: f32,
    line_thickness: f32,
    point_radius: f32,
}

#[derive(Serialize, Deserialize)]
struct Parameter {
    name: String,
    type_: ParameterType,
    value: Multivector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

        cc.storage
            .unwrap()
            .get_string("App")
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn update_code(&mut self) {
        let mut assigned_variables = HashSet::new();

        for parameter in &self.parameters {
            self.variables
                .entry(parameter.name.clone())
                .or_insert_with(|| Variable {
                    value: Multivector::ZERO,
                    display: None,
                })
                .value = parameter.value;
            assigned_variables.insert(parameter.name.as_str());
        }

        self.errors.clear();
        'evaluation: {
            let statements = match parse(&self.code) {
                Ok(statements) => statements,
                Err(error) => {
                    self.errors.push(format!("{error}"));
                    break 'evaluation;
                }
            };

            for statement in statements {
                match statement.kind {
                    AstStatementKind::Assignment {
                        name,
                        name_token: _,
                        equals_token: _,
                        value,
                    } => {
                        let value = match evaluate_expression(&value, &self.variables) {
                            Ok(value) => value,
                            Err(error) => {
                                self.errors.push(error);
                                continue;
                            }
                        };
                        self.variables
                            .entry(name.into())
                            .or_insert_with(|| Variable {
                                value: Multivector::ZERO,
                                display: None,
                            })
                            .value = value;
                        assigned_variables.insert(name);
                    }
                }
            }
        }

        if self.errors.is_empty() {
            self.variables
                .retain(|variable_name, _| assigned_variables.contains(variable_name.as_str()));
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let mut code_or_parameters_changed = self.last_time.is_none(); // hacky way to detect first time code has run

        let time = std::time::Instant::now();
        let dt = (time - self.last_time.unwrap_or(time)).as_secs_f32();
        self.last_time = Some(time);

        egui::TopBottomPanel::top("Menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.info_window_open |= ui.button("Info").clicked();
                self.camera_window_open |= ui.button("Camera").clicked();
                self.parameters_window_open |= ui.button("Parameters").clicked();
                self.code_window_open |= ui.button("Code").clicked();
                self.variables_window_open |= ui.button("Variables Window").clicked();
            });
        });

        {
            let mut reset_everything = false;
            egui::Window::new("Info")
                .open(&mut self.info_window_open)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("FPS: {:.3}", 1.0 / dt));
                    ui.label(format!("Frame Time: {:.3}ms", 1000.0 * dt));
                    reset_everything |= ui.button("RESET EVERYTHING").clicked();
                });
            if reset_everything {
                *self = Self::default();
                return;
            }
        }

        egui::Window::new("Camera")
            .open(&mut self.camera_window_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.collapsing("Transform", |ui| {
                    edit_multivector(ui, &mut self.camera.transform, true, true, true, true);
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
                    ui.label("Line Thickness:");
                    ui.add(egui::DragValue::new(&mut self.camera.line_thickness).speed(0.01));
                });
                ui.horizontal(|ui| {
                    ui.label("Point Radius:");
                    ui.add(egui::DragValue::new(&mut self.camera.point_radius).speed(0.01));
                });
            });

        egui::Window::new("Parameters")
            .open(&mut self.parameters_window_open)
            .resizable(true)
            .show(ctx, |ui| {
                if ui.button("New Parameter").clicked() {
                    self.parameters.push(Parameter {
                        name: "unnamed".into(),
                        type_: ParameterType::Grade0,
                        value: Multivector::ZERO,
                    });
                    code_or_parameters_changed = true;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut i = 0usize;
                    let mut delete = false;
                    self.parameters.retain_mut(|parameter| {
                        egui::CollapsingHeader::new(&parameter.name)
                            .id_salt(i)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Name:");
                                    code_or_parameters_changed |=
                                        ui.text_edit_singleline(&mut parameter.name).changed();
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Type:");
                                    if egui::ComboBox::from_id_salt("type")
                                        .selected_text(parameter.type_.display_name())
                                        .show_ui(ui, |ui| {
                                            let mut changed = false;
                                            for type_ in [
                                                ParameterType::Grade0,
                                                ParameterType::Grade1,
                                                ParameterType::Grade2,
                                                ParameterType::Grade3,
                                                ParameterType::Multivector,
                                            ] {
                                                changed |= ui
                                                    .selectable_value(
                                                        &mut parameter.type_,
                                                        type_,
                                                        type_.display_name(),
                                                    )
                                                    .changed();
                                            }
                                            changed
                                        })
                                        .inner
                                        .unwrap_or(false)
                                    {
                                        parameter.value = match parameter.type_ {
                                            ParameterType::Grade0 => parameter.value.grade0(),
                                            ParameterType::Grade1 => parameter.value.grade1(),
                                            ParameterType::Grade2 => parameter.value.grade2(),
                                            ParameterType::Grade3 => parameter.value.grade3(),
                                            ParameterType::Multivector => parameter.value,
                                        };
                                        code_or_parameters_changed = true;
                                    }
                                });

                                if ui.button("Normalise").clicked() {
                                    parameter.value = parameter.value.normalized();
                                }

                                let (grade0, grade1, grade2, grade3) = match parameter.type_ {
                                    ParameterType::Grade0 => (true, false, false, false),
                                    ParameterType::Grade1 => (false, true, false, false),
                                    ParameterType::Grade2 => (false, false, true, false),
                                    ParameterType::Grade3 => (false, false, false, true),
                                    ParameterType::Multivector => (true, true, true, true),
                                };

                                code_or_parameters_changed |= edit_multivector(
                                    ui,
                                    &mut parameter.value,
                                    grade0,
                                    grade1,
                                    grade2,
                                    grade3,
                                );

                                delete = ui.button("Delete").clicked();
                                code_or_parameters_changed |= delete;
                            });

                        i += 1;
                        !delete
                    });
                    ui.allocate_space(ui.available_size());
                });
            });

        egui::Window::new("Code")
            .open(&mut self.code_window_open)
            .scroll(true)
            .show(ctx, |ui| {
                if !self.errors.is_empty() {
                    ui.heading("Errors:");
                    for error in &self.errors {
                        ui.label(egui::RichText::new(error).color(egui::Color32::RED));
                    }
                }
                code_or_parameters_changed |= ui
                    .add(
                        egui::TextEdit::multiline(&mut self.code)
                            .id_salt("code")
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .min_size(ui.available_size()),
                    )
                    .changed();
            });

        if code_or_parameters_changed {
            self.update_code();
        }

        egui::Window::new("Variables")
            .open(&mut self.variables_window_open)
            .scroll([false, true])
            .show(ctx, |ui| {
                for (name, variable) in &mut self.variables {
                    let color = variable.display.as_ref().map(|display| {
                        egui::Color32::from_rgb(
                            (display.color.x * 255.0) as u8,
                            (display.color.y * 255.0) as u8,
                            (display.color.z * 255.0) as u8,
                        )
                    });
                    egui::CollapsingHeader::new(
                        egui::RichText::new(name).color(color.unwrap_or(egui::Color32::WHITE)),
                    )
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Display:");
                            let mut display_enabled = variable.display.is_some();
                            if ui.checkbox(&mut display_enabled, "").changed() {
                                if display_enabled {
                                    variable.display = Some(VariableDisplay {
                                        color: cgmath::Vector3 {
                                            x: 1.0,
                                            y: 1.0,
                                            z: 1.0,
                                        },
                                        layer: 0.05,
                                    });
                                } else {
                                    variable.display = None;
                                }
                            }
                        });

                        if let Some(display) = &mut variable.display {
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                ui.color_edit_button_rgb(display.color.as_mut());
                            });

                            ui.horizontal(|ui| {
                                ui.label("Layer");
                                ui.add(egui::Slider::new(&mut display.layer, 0.0..=1.0));
                            });
                        }

                        ui.collapsing("Value", |ui| {
                            ui.add_enabled_ui(false, |ui| {
                                edit_multivector(ui, &mut variable.value, true, true, true, true);
                            });
                        });
                    });
                }
            });

        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                let mut move_direction = cgmath::Vector2 { x: 0.0, y: 0.0 };
                move_direction.y += i.key_down(egui::Key::W) as u8 as f32;
                move_direction.y -= i.key_down(egui::Key::S) as u8 as f32;
                move_direction.x -= i.key_down(egui::Key::A) as u8 as f32;
                move_direction.x += i.key_down(egui::Key::D) as u8 as f32;

                let inf_point = Multivector {
                    e1: move_direction.x,
                    e2: move_direction.y,
                    ..Multivector::ZERO
                }
                .wedge(Multivector {
                    e0: 1.0,
                    ..Multivector::ZERO
                });

                let motor = Multivector::exp(
                    inf_point.normalized()
                        * (self.camera.move_speed * self.camera.view_height * dt * 0.5),
                );

                self.camera.transform = self.camera.transform * motor;

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

                let mut objects = vec![];

                for variable in self.variables.values() {
                    if let Some(display) = &variable.display {
                        objects.push(GpuObject {
                            value: variable.value,
                            color: display.color,
                            layer: display.layer,
                        });
                    }
                }

                self.camera.view_height = self.camera.view_height.max(0.1);
                ui.painter()
                    .add(eframe::egui_wgpu::Callback::new_paint_callback(
                        rect,
                        RenderData {
                            camera: GpuCamera {
                                transform: self.camera.transform,
                                vertical_height: self.camera.view_height,
                                aspect,
                                line_thickness: self.camera.line_thickness,
                                point_radius: self.camera.point_radius,
                            },
                            objects,
                        },
                    ));
            });

        ctx.request_repaint();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("App", ron::to_string(self).unwrap());
    }
}

fn edit_multivector(
    ui: &mut egui::Ui,
    value: &mut Multivector,
    grade0: bool,
    grade1: bool,
    grade2: bool,
    grade3: bool,
) -> bool {
    let mut changed = false;
    if grade0 {
        ui.horizontal(|ui| {
            ui.label("Scalar:");
            changed |= ui
                .add(egui::DragValue::new(&mut value.s).speed(0.1))
                .changed();
        });
    }
    if grade1 {
        ui.horizontal(|ui| {
            ui.label("e0:");
            changed |= ui
                .add(egui::DragValue::new(&mut value.e0).speed(0.1))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("e1:");
            changed |= ui
                .add(egui::DragValue::new(&mut value.e1).speed(0.1))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("e2:");
            changed |= ui
                .add(egui::DragValue::new(&mut value.e2).speed(0.1))
                .changed();
        });
    }
    if grade2 {
        ui.horizontal(|ui| {
            ui.label("e01:");
            changed |= ui
                .add(egui::DragValue::new(&mut value.e01).speed(0.1))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("e02:");
            changed |= ui
                .add(egui::DragValue::new(&mut value.e02).speed(0.1))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("e12:");
            changed |= ui
                .add(egui::DragValue::new(&mut value.e12).speed(0.1))
                .changed();
        });
    }
    if grade3 {
        ui.horizontal(|ui| {
            ui.label("e012:");
            changed |= ui
                .add(egui::DragValue::new(&mut value.e012).speed(0.1))
                .changed();
        });
    }
    changed
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Geometric Algebra Sandbox",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            vsync: false,
            wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
                present_mode: wgpu::PresentMode::AutoNoVsync,
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
