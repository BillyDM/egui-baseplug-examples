#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(min_specialization)]

use baseplug::{Plugin, ProcessContext, WindowOpenResult, UIModel, Model, UIFloatParam};
use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
use raw_window_handle::HasRawWindowHandle;
use serde::{Deserialize, Serialize};

use egui::CtxRef;
use egui_baseview::{EguiWindow, Queue, RenderSettings, Settings};

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct GainModel {
        #[model(min = -90.0, max = 3.0)]
        #[parameter(name = "gain left", unit = "Decibels",
            gradient = "Power(0.15)")]
        gain_left: f32,

        #[model(min = -90.0, max = 3.0)]
        #[parameter(name = "gain right", unit = "Decibels",
            gradient = "Power(0.15)")]
        gain_right: f32,

        #[model(min = -90.0, max = 3.0)]
        #[parameter(name = "gain master", unit = "Decibels",
            gradient = "Power(0.15)")]
        gain_master: f32,

        non_parameter_value_test: f32,
    }
}

impl Default for GainModel {
    fn default() -> Self {
        Self {
            // "gain" is converted from dB to coefficient in the parameter handling code,
            // so in the model here it's a coeff.
            // -0dB == 1.0
            gain_left: 1.0,
            gain_right: 1.0,
            gain_master: 1.0,

            non_parameter_value_test: 1.0,
        }
    }
}

struct Gain {}

impl Plugin for Gain {
    const NAME: &'static str = "egui-baseplug gain";
    const PRODUCT: &'static str = "egui-baseplug gain";
    const VENDOR: &'static str = "spicy plugins & co";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = GainModel;

    #[inline]
    fn new(_sample_rate: f32, _model: &GainModel) -> Self {
        Self {}
    }

    #[inline]
    fn process(&mut self, model: &GainModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;

        for i in 0..ctx.nframes {
            output[0][i] = input[0][i] * model.gain_left[i] * model.gain_master[i] * model.non_parameter_value_test[i];
            output[1][i] = input[1][i] * model.gain_right[i] * model.gain_master[i] * model.non_parameter_value_test[i];
        }
    }
}

impl baseplug::PluginUI for Gain {
    type Handle = ();

    fn ui_size() -> (i16, i16) {
        (500, 300)
    }

    fn ui_open(parent: &impl HasRawWindowHandle, model: <Self::Model as Model<Self>>::UI) -> WindowOpenResult<Self::Handle> {
        let settings = Settings {
            window: WindowOpenOptions {
                title: String::from("egui-baseplug-examples gain"),
                size: Size::new(Self::ui_size().0 as f64, Self::ui_size().1 as f64),
                scale: WindowScalePolicy::SystemScaleFactor,
            },
            render_settings: RenderSettings::default(),
        };

        let state = State::new(model);

        EguiWindow::open_parented(
            parent,
            settings,
            state,
            // Called once before the first frame. Allows you to do setup code and to
            // call `ctx.set_fonts()`. Optional.
            |_egui_ctx: &CtxRef, _queue: &mut Queue, _state: &mut State| {},
            // Called before each frame. Here you should update the state of your
            // application and build the UI.
            |egui_ctx: &CtxRef, _queue: &mut Queue, state: &mut State| {
                // Must be called on the top of each frame in order to sync values from the rt thread.
                state.model.poll_updates();

                let format_value = |value_text: &mut String, param: &UIFloatParam<_, _>| {
                    *value_text = format!("{:.1} {}", param.unit_value(), param.unit_label());
                };

                let update_value_text = |value_text: &mut String, param: &UIFloatParam<_, _>| {
                    if param.updated_by_host() {
                        format_value(value_text, param)
                    }
                };

                let param_slider = |ui: &mut egui::Ui, label: &str, value_text: &mut String, param: &mut UIFloatParam<_, _>| {
                    ui.label(label);
                    ui.label(value_text.as_str());

                    // Use the normalized value of the param so we can take advantage of baseplug's value curves.
                    //
                    // You could opt to use your own custom widget if you wish, as long as it can operate with
                    // a normalized range from [0.0, 1.0].
                    let mut normal = param.normalized();
                    if ui.add(egui::Slider::new(&mut normal, 0.0..=1.0)).changed() {
                        param.set_from_normalized(normal);
                        format_value(value_text, param);
                    };
                };

                // Sync text values if there was automation.
                update_value_text(&mut state.gain_master_value, &state.model.gain_master);
                update_value_text(&mut state.gain_left_value, &state.model.gain_left);
                update_value_text(&mut state.gain_right_value, &state.model.gain_right);

                egui::Window::new("egui-baseplug gain demo").show(&egui_ctx, |ui| {
                    param_slider(ui, "Gain Master", &mut state.gain_master_value, &mut state.model.gain_master);
                    param_slider(ui, "Gain Left", &mut state.gain_left_value, &mut state.model.gain_left);
                    param_slider(ui, "Gain Right", &mut state.gain_right_value, &mut state.model.gain_right);

                    ui.label("non-parameter value test");
                    let mut value = state.model.non_parameter_value_test.get();
                    if ui.add(egui::Slider::new(&mut value, 0.0..=1.0)).changed() {
                        state.model.non_parameter_value_test.set(value);
                    };
                });

                // TODO: Add a way for egui-baseview to send a closure that runs every frame without always
                // repainting.
                egui_ctx.request_repaint();
            },
        );

        Ok(())
    }

    fn ui_close(mut handle: Self::Handle) {
        // TODO: Close window once baseview gets the ability to do this.
    }
}

struct State {
    model: GainModelUI<Gain>,

    gain_master_value: String,
    gain_left_value: String,
    gain_right_value: String,
}

impl State {
    pub fn new(model: GainModelUI<Gain>) -> State {
        State {
            model,
            gain_master_value: String::new(),
            gain_left_value: String::new(),
            gain_right_value: String::new(),
        }
    }
}

baseplug::vst2!(Gain, b"tAnE");