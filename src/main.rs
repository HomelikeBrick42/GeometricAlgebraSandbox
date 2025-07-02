use crate::{
    multivector::Multivector,
    parsing::{AstExpression, AstExpressionKind, AstStatementKind, BinaryOperator, parse},
    rendering::{GpuCamera, GpuCircle, GpuQuad, RenderData, RenderState},
};
use eframe::{egui, wgpu};
use std::collections::HashMap;

pub mod lexer;
pub mod multivector;
pub mod parsing;
pub mod rendering;

struct App {
    last_time: Option<std::time::Instant>,
    info_window_open: bool,
    camera_window_open: bool,
    camera: Camera,
    parameters_window_open: bool,
    parameters: Vec<Parameter>,
    code_window_open: bool,
    errors: Vec<String>,
    code: String,
    values_to_display_window_open: bool,
    values_to_display: Vec<ValueToDisplay>,
    variables: HashMap<String, Multivector>,
}

struct Camera {
    position: cgmath::Vector2<f32>,
    view_height: f32,
    move_speed: f32,
    zoom_speed: f32,
    show_grid: bool,
    grid_thickness: f32,
    line_thickness: f32,
    point_radius: f32,
}

struct Parameter {
    name: String,
    type_: ParameterType,
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

struct ValueToDisplay {
    name: String,
    color: cgmath::Vector3<f32>,
    layer: f32,
    display_value: Multivector,
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
            values_to_display_window_open: true,
            values_to_display: vec![
                ValueToDisplay {
                    name: "e1".into(),
                    color: cgmath::Vector3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    layer: 0.0,
                    display_value: Multivector::ZERO,
                },
                ValueToDisplay {
                    name: "e2".into(),
                    color: cgmath::Vector3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                    layer: 0.0,
                    display_value: Multivector::ZERO,
                },
                ValueToDisplay {
                    name: "e12".into(),
                    color: cgmath::Vector3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                    layer: 0.0,
                    display_value: Multivector::ZERO,
                },
            ],
            variables: HashMap::new(),
        }
    }

    fn update_code(&mut self) {
        self.variables.clear();
        for parameter in &self.parameters {
            self.variables
                .insert(parameter.name.clone(), parameter.value);
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
                fn evaluate_value(
                    expression: &AstExpression,
                    variables: &HashMap<String, Multivector>,
                ) -> Result<Multivector, String> {
                    Ok(match expression.kind {
                        AstExpressionKind::Name {
                            name,
                            ref name_token,
                        } => match variables.get(name) {
                            Some(value) => *value,
                            None => {
                                return Err(format!(
                                    "{}: Unknown variable '{name}'",
                                    name_token.location
                                ));
                            }
                        },
                        AstExpressionKind::Number {
                            number,
                            number_token: _,
                        } => Multivector {
                            s: number,
                            ..Multivector::ZERO
                        },
                        AstExpressionKind::Unary {
                            ref operator,
                            operator_token: _,
                            ref operand,
                        } => {
                            let operand = evaluate_value(operand, variables)?;
                            match operator {
                                parsing::UnaryOperator::Negate => -operand,
                                parsing::UnaryOperator::Dual => operand.dual(),
                                parsing::UnaryOperator::Reverse => operand.reverse(),
                            }
                        }
                        AstExpressionKind::Binary {
                            ref left,
                            ref operator,
                            ref operator_token,
                            ref right,
                        } => {
                            let left = evaluate_value(left, variables)?;
                            let right = evaluate_value(right, variables)?;
                            match operator {
                                BinaryOperator::Add => left + right,
                                BinaryOperator::Subtract => left - right,
                                BinaryOperator::Multiply => left * right,
                                BinaryOperator::Divide => {
                                    return Err(format!(
                                        "{}: Divide unimplemented",
                                        operator_token.location
                                    ));
                                }
                                BinaryOperator::Wedge => left.wedge(right),
                                BinaryOperator::Inner => left.inner(right),
                                BinaryOperator::Regressive => left.regressive(right),
                            }
                        }
                    })
                }

                match statement.kind {
                    AstStatementKind::Assignment {
                        name,
                        name_token: _,
                        equals_token: _,
                        value,
                    } => {
                        let value = match evaluate_value(&value, &self.variables) {
                            Ok(value) => value,
                            Err(error) => {
                                self.errors.push(error);
                                break 'evaluation;
                            }
                        };
                        self.variables.insert(name.into(), value);
                    }
                }
            }
        }

        for values_to_display in &mut self.values_to_display {
            values_to_display.display_value = self
                .variables
                .get(&values_to_display.name)
                .copied()
                .unwrap_or(Multivector::ZERO);
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
                self.values_to_display_window_open |= ui.button("Values To Display").clicked();
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
            .scroll([false, true])
            .show(ctx, |ui| {
                if ui.button("New Parameter").clicked() {
                    self.parameters.push(Parameter {
                        name: "unnamed".into(),
                        type_: ParameterType::Grade0,
                        value: Multivector::ZERO,
                    });
                    code_or_parameters_changed = true;
                }
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
                                parameter.value = parameter.value.normalised();
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

        egui::Window::new("Values To Display")
            .open(&mut self.values_to_display_window_open)
            .scroll([false, true])
            .show(ctx, |ui| {
                if ui.button("New Value To Display").clicked() {
                    self.values_to_display.push(ValueToDisplay {
                        name: "unassigned".into(),
                        color: cgmath::Vector3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        layer: 0.0,
                        display_value: Multivector::ZERO,
                    });
                    code_or_parameters_changed |= true;
                }
                let mut i = 0usize;
                let mut delete = false;
                self.values_to_display.retain_mut(|value_to_display| {
                    egui::CollapsingHeader::new(egui::RichText::new(&value_to_display.name).color(
                        egui::Color32::from_rgb(
                            (value_to_display.color.x * 255.0) as u8,
                            (value_to_display.color.y * 255.0) as u8,
                            (value_to_display.color.z * 255.0) as u8,
                        ),
                    ))
                    .id_salt(i)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            code_or_parameters_changed |= ui
                                .text_edit_singleline(&mut value_to_display.name)
                                .changed();
                        });

                        ui.horizontal(|ui| {
                            ui.label("Color:");
                            ui.color_edit_button_rgb(value_to_display.color.as_mut());
                        });

                        ui.horizontal(|ui| {
                            ui.label("Layer");
                            ui.add(egui::Slider::new(&mut value_to_display.layer, 0.0..=1.0));
                        });

                        ui.add_enabled_ui(false, |ui| {
                            edit_multivector(
                                ui,
                                &mut value_to_display.display_value,
                                true,
                                true,
                                true,
                                true,
                            );
                        });

                        delete = ui.button("Delete").clicked();
                        code_or_parameters_changed |= delete;
                    });

                    i += 1;
                    !delete
                });
            });

        if code_or_parameters_changed {
            self.update_code();
        }

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

                let mut quads = vec![];
                let mut circles = vec![];

                if self.camera.show_grid {
                    let view_width = self.camera.view_height * aspect;
                    for i in 0..view_width.ceil() as usize + 2 {
                        let position =
                            (i as f32 - view_width * 0.5 - 1.0 + self.camera.position.x).round();
                        quads.push(GpuQuad {
                            position: cgmath::Vector3 {
                                x: position,
                                y: self.camera.position.y,
                                z: 0.0,
                            },
                            rotation: 0.0,
                            color: cgmath::Vector3 {
                                x: 0.5,
                                y: 0.5,
                                z: 0.5,
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
                                z: 0.0,
                            },
                            rotation: 0.0,
                            color: cgmath::Vector3 {
                                x: 0.5,
                                y: 0.5,
                                z: 0.5,
                            },
                            size: cgmath::Vector2 {
                                x: view_width,
                                y: self.camera.grid_thickness,
                            },
                        });
                    }
                }

                for value_to_display in &self.values_to_display {
                    let line = value_to_display.display_value.grade1().normalised();
                    if line.sqr_magnitude() > 0.0001 {
                        quads.push(GpuQuad {
                            position: cgmath::Vector3 {
                                x: line.e1 * -line.e0,
                                y: line.e2 * -line.e0,
                                z: value_to_display.layer,
                            },
                            rotation: f32::atan2(line.e1, line.e2),
                            color: value_to_display.color,
                            size: cgmath::Vector2 {
                                x: 10000.0,
                                y: self.camera.line_thickness,
                            },
                        });
                    }

                    let point = value_to_display.display_value.grade2().normalised();
                    if point.sqr_magnitude() > 0.0001 {
                        circles.push(GpuCircle {
                            position: cgmath::Vector3 {
                                x: -point.e02,
                                y: point.e01,
                                z: value_to_display.layer,
                            },
                            color: value_to_display.color,
                            radius: self.camera.point_radius,
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
