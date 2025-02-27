use serde::Deserialize;
use std::{collections::HashSet, path::Path};

use plojo_core::{Command, Controller, Machine, Stroke};
use plojo_input_geminipr::GeminiprMachine;
use plojo_input_stdin::StdinMachine;
use plojo_output_wayland::WaylandController;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    input_machine: InputMachineType,
    #[serde(default)]
    output_dispatcher: OutputDispatchType,
    #[serde(default)]
    dicts: Vec<String>,
    #[serde(default)]
    retrospective_add_space_strokes: Vec<String>,
    #[serde(default)]
    space_stroke: Option<String>,
    #[serde(default)]
    pub space_after: bool,
    #[serde(default)]
    pub delay_output: bool,
    #[serde(default)]
    disable_input_strokes: Vec<String>,
    #[serde(default)]
    enable_input_shortcuts: Vec<Vec<String>>,
    #[serde(default)]
    disable_scan_keymap: bool,
}

impl Config {
    /// Creates an input machine from the config. Can panic if failed to create machine.
    /// Accepts an override to ignore config and use stdin
    pub fn get_input_machine(&self, use_stdin: bool) -> Box<dyn Machine> {
        let input = if use_stdin {
            println!("[INFO] Overriding config to use input from stdin");
            &InputMachineType::Stdin
        } else {
            &self.input_machine
        };
        println!("[INFO] Input from: {:?}", input);
        match input {
            InputMachineType::Stdin => Box::new(StdinMachine::new()) as Box<dyn Machine>,
            InputMachineType::Geminipr { ref port } => {
                Box::new(GeminiprMachine::new(port).expect("unable to connect to geminipr machine"))
                    as Box<dyn Machine>
            }
            InputMachineType::Keyboard => unreachable!(),
        }
    }

    /// Create an output controller from the config
    /// Accepts an override to ignore config and use stdout
    pub fn get_output_controller(&self, use_stdout: bool) -> Box<dyn Controller> {
        let output = if use_stdout {
            println!("[INFO] Overriding config to output to stdout");
            &OutputDispatchType::Stdout
        } else {
            &self.output_dispatcher
        };
        println!("[INFO] Output to: {:?}", output);
        match output {
            OutputDispatchType::Enigo => {
                panic!()
            }
            OutputDispatchType::MacNative => {
                panic!()
            }
            OutputDispatchType::Stdout => {
                Box::new(StdoutController::new(self.disable_scan_keymap)) as Box<dyn Controller>
            }
            OutputDispatchType::Wayland => {
                Box::new(WaylandController::new(self.disable_scan_keymap)) as Box<dyn Controller>
            }
        }
    }

    /// Read dictionary files with the path from the config given the base path to them
    pub fn get_dicts(&self, base_path: &Path) -> Vec<String> {
        self.dicts
            .iter()
            .map(|p| base_path.join(&p))
            .map(|p| {
                println!("[INFO] Loading {:?}", p);
                match std::fs::read_to_string(&p) {
                    Ok(s) => s,
                    Err(e) => panic!("unable to read dictionary file {:?}: {:?}", p, e),
                }
            })
            .collect()
    }

    /// Get the strokes for retrospective add space
    pub fn get_retro_add_space(&self) -> Vec<Stroke> {
        self.retrospective_add_space_strokes
            .iter()
            .map(|s| Stroke::new(s))
            .collect()
    }

    /// Get the stroke for space that is added when retrospectively adding space
    pub fn get_space_stroke(&self) -> Option<Stroke> {
        self.space_stroke.as_ref().map(|s| Stroke::new(s))
    }

    /// Get the strokes for disabling input (mainly for keyboard input)
    pub fn get_disable_input_strokes(&self) -> HashSet<Stroke> {
        self.disable_input_strokes
            .iter()
            .map(|s| Stroke::new(s))
            .collect::<HashSet<_>>()
    }
}

pub fn load(raw_str: &str) -> Result<Config, toml::de::Error> {
    toml::from_str::<Config>(raw_str)
}

#[derive(Debug, Deserialize)]
enum InputMachineType {
    Stdin,
    Keyboard,
    Geminipr { port: String },
}

impl Default for InputMachineType {
    fn default() -> Self {
        Self::Stdin
    }
}

#[derive(Debug, Deserialize)]
enum OutputDispatchType {
    MacNative,
    Enigo,
    Stdout,
    Wayland,
}

impl Default for OutputDispatchType {
    fn default() -> Self {
        Self::Stdout
    }
}

struct StdoutController {}
impl Controller for StdoutController {
    fn new(_disable_scan_keymap: bool) -> Self {
        Self {}
    }
    fn dispatch(&mut self, command: Command) {
        println!("{:?}", command);
    }
}
