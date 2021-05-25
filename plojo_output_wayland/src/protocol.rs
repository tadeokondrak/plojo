#![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
#![allow(non_upper_case_globals, non_snake_case, unused_imports)]

pub mod zwp_input_method_v2 {
    pub mod client {
        use super::super::zwp_text_input_v3::client::zwp_text_input_v3;
        use wayland_client::protocol::*;
        use wayland_client::sys;
        use wayland_client::{AnonymousObject, Attached, Main, Proxy, ProxyMap};
        use wayland_commons::map::{Object, ObjectMetadata};
        use wayland_commons::smallvec;
        use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
        use wayland_commons::{Interface, MessageGroup};
        include!(concat!(env!("OUT_DIR"), "/input_method_unstable_v2.rs"));
    }
}

pub mod zwp_text_input_v3 {
    pub mod client {
        use wayland_client::protocol::*;
        use wayland_client::sys;
        use wayland_client::{AnonymousObject, Attached, Main, Proxy, ProxyMap};
        use wayland_commons::map::{Object, ObjectMetadata};
        use wayland_commons::smallvec;
        use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
        use wayland_commons::{Interface, MessageGroup};
        include!(concat!(env!("OUT_DIR"), "/text_input_unstable_v3.rs"));
    }
}

pub mod zwp_virtual_keyboard_v1 {
    pub mod client {
        use wayland_client::protocol::*;
        use wayland_client::sys;
        use wayland_client::{AnonymousObject, Attached, Main, Proxy, ProxyMap};
        use wayland_commons::map::{Object, ObjectMetadata};
        use wayland_commons::smallvec;
        use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
        use wayland_commons::{Interface, MessageGroup};
        include!(concat!(env!("OUT_DIR"), "/virtual_keyboard_unstable_v1.rs"));
    }
}
