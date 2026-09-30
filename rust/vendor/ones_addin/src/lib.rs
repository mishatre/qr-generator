pub mod ffi;
pub mod utf16;

pub trait OnesAddIn {
    fn hello_macro();
}

#[macro_export]
macro_rules! match_index {
    ($e:tt, $($result:tt),+) => {{
        let mut i = 0..;
        let e = $e;
        $(
            if e == i.next().unwrap() {
                let add_in_component = $result::new();
                create_component(component, add_in_component);
            } else
        )*
        { unreachable!() }
    }}
}

#[macro_export]
macro_rules! export_addin {
    ($arg:tt) => {

        use std::{
            ffi::{c_int, c_long, c_void},
            sync::atomic::{AtomicI32, Ordering},
        };

        use ::ones_addin::match_index;

        use ::ones_addin::ffi::AttachType;
        use ::ones_addin::ffi::{create_component, destroy_component};

        pub static mut PLATFORM_CAPABILITIES: AtomicI32 = AtomicI32::new(-1);

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn GetClassObject(
            name: *const u16,
            component: *mut *mut c_void,
        ) -> c_long {
            // The platform passes back a class id from `GetClassNames`. A library
            // with one component has nothing to dispatch on, so the name is
            // deliberately not read; the object type the script uses is
            // `AddIn.<connection>.<register_extension_as name>`.
            let _ = name;
            let add_in_component = $arg::new();
            create_component(component, add_in_component)
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn DestroyObject(component: *mut *mut c_void) -> c_long {
            destroy_component(component)
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub extern "C" fn GetClassNames() -> *const u16 {
            // small strings for performance
            ones_addin::utf16::utf16_null!("1").as_ptr()
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn SetPlatformCapabilities(capabilities: c_int) -> c_int {
            PLATFORM_CAPABILITIES.store(capabilities, Ordering::Relaxed);
            3
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub extern "C" fn GetAttachType() -> AttachType {
            AttachType::CanAttachAny
        }

    };
    ($($arg:tt),+) => {
        use std::{
            ffi::{c_int, c_long, c_void},
            sync::atomic::{AtomicI32, Ordering},
        };

        use ::ones_addin::match_index;

        use ::ones_addin::ffi::AttachType;
        use ::ones_addin::ffi::{create_component, destroy_component};

        pub static mut PLATFORM_CAPABILITIES: AtomicI32 = AtomicI32::new(-1);

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn GetClassObject(
            name: *const u16,
            component: *mut *mut c_void,
        ) -> c_long {
            ::ones_addin::match_index!($( $arg ),+);
            0
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn DestroyObject(component: *mut *mut c_void) -> c_long {
            destroy_component(component)
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub extern "C" fn GetClassNames() -> *const u16 {
            let mut index: i32 = 0;
            let mut class_names: Vec<String> = vec![];

            // $(
            //     index += 1;
            //     // $arg;
            //     class_names.push(i32::to_string(&index));
            // )*

            // let s: Vec<u16> = class_names.join("|").encode_utf16().collect();
            // s.as_ptr()

            // small strings for performance
            ones_addin::utf16::utf16_null!("1|2").as_ptr()
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn SetPlatformCapabilities(capabilities: c_int) -> c_int {
            PLATFORM_CAPABILITIES.store(capabilities, Ordering::Relaxed);
            3
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub extern "C" fn GetAttachType() -> AttachType {
            AttachType::CanAttachAny
        }
    };
}
