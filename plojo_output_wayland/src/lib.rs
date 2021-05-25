use plojo_core::{Command, Controller};
use protocol::zwp_input_method_v2::client::zwp_input_method_manager_v2::ZwpInputMethodManagerV2;
use protocol::zwp_input_method_v2::client::zwp_input_method_v2::Event as ImEvent;
use protocol::zwp_input_method_v2::client::zwp_input_method_v2::ZwpInputMethodV2;
use protocol::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1;
use protocol::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1;
use std::process::Command as ProcessCommand;
use wayland_client::{protocol::wl_seat::WlSeat, Display, EventQueue, GlobalManager, Main};

#[macro_use]
extern crate bitflags;

mod protocol;

pub struct WaylandController {
    display: Display,
    queue: Option<EventQueue>,
    seat: Main<WlSeat>,
    virtual_keyboard: Main<ZwpVirtualKeyboardV1>,
    input_method: Main<ZwpInputMethodV2>,
    serial: u32,
}

impl Controller for WaylandController {
    fn new(_disable_scan_keymap: bool) -> Self {
        let display = Display::connect_to_env().unwrap();
        let mut queue = display.create_event_queue();
        let attached = (*display).clone().attach(queue.token());
        let manager = GlobalManager::new(&attached);
        queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();
        let seat = manager.instantiate_exact::<WlSeat>(1).unwrap();
        let virtual_keyboard_manager = manager
            .instantiate_exact::<ZwpVirtualKeyboardManagerV1>(1)
            .unwrap();
        let input_method_manager = manager
            .instantiate_exact::<ZwpInputMethodManagerV2>(1)
            .unwrap();
        let virtual_keyboard = virtual_keyboard_manager.create_virtual_keyboard(&seat);
        let input_method = input_method_manager.get_input_method(&seat);
        input_method.quick_assign(|input_method, event, mut data| {
            let data: &mut Self = data.get().unwrap();
            match event {
                ImEvent::Activate => {}
                ImEvent::Deactivate => {}
                ImEvent::SurroundingText {
                    text,
                    cursor,
                    anchor,
                } => {}
                ImEvent::TextChangeCause { cause } => {}
                ImEvent::ContentType { hint, purpose } => {}
                ImEvent::Done => data.serial = data.serial.wrapping_add(1),
                ImEvent::Unavailable => panic!("input method unavailable"),
            }
        });
        Self {
            display,
            queue: Some(queue),
            seat,
            virtual_keyboard,
            input_method,
            serial: 0,
        }
    }

    fn dispatch(&mut self, command: Command) {
        if let Some(guard) = self.queue.as_ref().unwrap().prepare_read() {
            if let Err(e) = guard.read_events() {
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    panic!(e);
                }
            }
        }
        {
            let mut queue = self.queue.take().unwrap();
            queue.dispatch_pending(self, |_, _, _| {}).unwrap();
            self.queue = Some(queue);
        }
        match command {
            Command::Replace(backspaces, text) => {
                self.input_method
                    .delete_surrounding_text(backspaces as u32, 0);
                self.input_method.commit_string(text);
                self.input_method.commit(self.serial);
            }
            Command::PrintHello => {
                println!("Hello!");
            }
            Command::NoOp => {}
            Command::Keys(_, _) => {}
            Command::Raw(_) => {}
            Command::Shell(cmd, args) => dispatch_shell(cmd, args),
            Command::TranslatorCommand(_) => panic!("cannot handle translator command"),
        }
        self.display.flush().unwrap();
    }
}

fn dispatch_shell(cmd: String, args: Vec<String>) {
    let result = ProcessCommand::new(cmd).args(args).spawn();
    match result {
        Ok(_) => {}
        Err(e) => eprintln!("[WARN] Could not execute shell command: {}", e),
    }
}
