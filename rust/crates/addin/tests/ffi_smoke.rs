//! Exercise the component the way the platform does: load the built library and
//! call its exported entry points.
//!
//! This is the only test outside 1C that touches the real ABI. The tests in
//! `generation.rs` link the crate as an `rlib` and never prove that the exported
//! `#[no_mangle]` entry points exist, that `GetClassObject` builds an object, or
//! what name the platform will see. Those are exactly the failures a 1C-licence-gated
//! test would otherwise be the first to notice.
//!
//! What it deliberately does not do is call a component *method*: that needs the
//! platform's own vtables driven from the other side, which `addin1c-test` or a
//! hand-built fake platform would provide. The in-platform suite under `tests/bsl/`
//! is the test that covers method calls — including the one thing nothing here can
//! check, which is that the platform presents the returned bytes to a script as
//! `ДвоичныеДанные` rather than as a string.
//!
//! The library has to exist before this test can do anything, so build it first:
//!
//! ```text
//! cargo build -p qr_generator_addin
//! cargo test -p qr_generator_addin --test ffi_smoke
//! ```
//!
//! When no library is found the test reports that and passes, so a plain
//! `cargo test` on a fresh checkout is not a false failure. CI and the documented
//! verification sequence build first, so the checks always run there.

use std::ffi::c_void;
use std::os::raw::c_long;
use std::path::{Path, PathBuf};

type GetClassNames = unsafe extern "C" fn() -> *const u16;
type GetClassObject = unsafe extern "C" fn(name: *const u16, component: *mut *mut c_void) -> c_long;
type DestroyObject = unsafe extern "C" fn(component: *mut *mut c_void) -> c_long;
type SetPlatformCapabilities = unsafe extern "C" fn(capabilities: i32) -> i32;
type GetAttachType = unsafe extern "C" fn() -> i32;

/// Candidate output locations, in the order a developer is likely to have built.
fn candidate_paths() -> Vec<PathBuf> {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");

    let file_name = if cfg!(target_os = "macos") {
        "libqr_generator_addin.dylib"
    } else if cfg!(target_os = "windows") {
        "qr_generator_addin.dll"
    } else {
        "libqr_generator_addin.so"
    };

    [
        "debug",
        "release",
        "x86_64-unknown-linux-gnu/debug",
        "x86_64-unknown-linux-gnu/release",
        // The host a developer is most likely to be on when running this by hand.
        "aarch64-apple-darwin/debug",
        "aarch64-apple-darwin/release",
    ]
    .iter()
    .map(|layout| target.join(layout).join(file_name))
    .collect()
}

fn built_library() -> Option<PathBuf> {
    candidate_paths().into_iter().find(|path| path.exists())
}

/// Read a null-terminated UTF-16 string, which is how the platform passes and
/// receives every name across this ABI. Two bytes per unit on Linux as well as on
/// Windows, whatever the platform's own `WCHAR_T` happens to be: the vendored
/// framework reads every name in the interface that way, and the in-platform suite
/// matches Russian method names through it. The reading is written out rather than
/// assumed because a wrong width here shows up in 1C as a missing method.
unsafe fn read_wide(pointer: *const u16) -> String {
    assert!(
        !pointer.is_null(),
        "the component returned a null name pointer"
    );

    let mut length = 0usize;
    while *pointer.add(length) != 0 {
        length += 1;
    }

    let slice = std::slice::from_raw_parts(pointer, length);
    String::from_utf16_lossy(slice)
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[test]
fn the_component_exports_the_entry_points_the_platform_looks_for() {
    let Some(path) = built_library() else {
        println!(
            "skipped: no built component found. Build one first with \
             `cargo build -p qr_generator_addin`."
        );
        return;
    };

    println!("checking {}", path.display());

    let library = unsafe { libloading::Library::new(&path) }
        .unwrap_or_else(|error| panic!("cannot load {}: {error}", path.display()));

    // Every one of these must exist. A missing symbol here is a missing symbol in
    // 1C, where the failure would be "component not found" with no further detail.
    for symbol in [
        "GetClassNames",
        "GetClassObject",
        "DestroyObject",
        "SetPlatformCapabilities",
        "GetAttachType",
    ] {
        let found = unsafe { library.get::<*const c_void>(symbol.as_bytes()) };

        assert!(
            found.is_ok(),
            "the component does not export {symbol}; 1C would refuse to load it"
        );
    }
}

#[test]
fn the_platform_sees_one_object() {
    let Some(path) = built_library() else {
        println!("skipped: no built component found");
        return;
    };

    let library = unsafe { libloading::Library::new(&path) }.expect("the library loads");
    let get_class_names = unsafe { library.get::<GetClassNames>(b"GetClassNames") }
        .expect("GetClassNames is exported");

    let names = unsafe { read_wide(get_class_names()) };

    // This is the identifier the platform passes to `GetClassObject`, the internal
    // class id, and not the name of the 1C object type: measured on 8.3.24.1667 in
    // the sibling project, the type the component is reachable as is
    // `AddIn.<имя подключения>.QrGenerator`. Pinned here because a crate upgrade
    // could change the id silently.
    assert_eq!(
        names, "1",
        "the object name the platform will use has changed; the BSL wrapper in \
         src/cfe/QrGenerator must be updated to match"
    );
}

#[test]
fn an_object_is_created_and_destroyed() {
    let Some(path) = built_library() else {
        println!("skipped: no built component found");
        return;
    };

    let library = unsafe { libloading::Library::new(&path) }.expect("the library loads");
    let get_class_object = unsafe { library.get::<GetClassObject>(b"GetClassObject") }
        .expect("GetClassObject is exported");
    let destroy_object = unsafe { library.get::<DestroyObject>(b"DestroyObject") }
        .expect("DestroyObject is exported");

    let name = wide("1");
    let mut component: *mut c_void = std::ptr::null_mut();

    let created = unsafe { get_class_object(name.as_ptr(), &mut component) };

    assert_ne!(created, 0, "GetClassObject refused to create the object");
    assert!(
        !component.is_null(),
        "GetClassObject returned a null object"
    );

    // The platform calls this for every object it created, so leaking here would
    // be a real leak in 1C. A non-zero result means the component failed to run
    // `Done`.
    let destroyed = unsafe { destroy_object(&mut component) };

    assert_eq!(destroyed, 0, "DestroyObject reported a failure");
}

/// The two answers the platform uses to decide how to talk to the object at all.
///
/// `SetPlatformCapabilities` gets the platform's own level and answers with the
/// highest the component supports; `GetAttachType` says the object may be attached
/// in both the isolated and the non-isolated way. Pinned because a vendored change
/// here would alter which interface layout the platform addresses the object
/// through, and that shows up in 1C as every method going missing.
#[test]
fn the_platform_is_told_the_interface_level_and_attach_type() {
    let Some(path) = built_library() else {
        println!("skipped: no built component found");
        return;
    };

    let library = unsafe { libloading::Library::new(&path) }.expect("the library loads");
    let set_capabilities =
        unsafe { library.get::<SetPlatformCapabilities>(b"SetPlatformCapabilities") }
            .expect("SetPlatformCapabilities is exported");
    let get_attach_type = unsafe { library.get::<GetAttachType>(b"GetAttachType") }
        .expect("GetAttachType is exported");

    let answer = unsafe { set_capabilities(3) };

    assert_eq!(
        answer, 3,
        "the component no longer answers with the Native API interface level 3"
    );

    // AttachType::CanAttachAny, which the platform reads as a C enum.
    assert_eq!(
        unsafe { get_attach_type() },
        3,
        "the attach type is no longer CanAttachAny"
    );
}

/// The runtime creates its single object whatever name it is handed, and the
/// platform only ever hands back the class id from `GetClassNames`.
///
/// Recorded as a test rather than an expectation: a name dispatch would have
/// nothing to choose between in a library with one component. If a vendored change
/// starts rejecting names, this fails here instead of in 1C, where the symptom is
/// a component that connects and then has no object.
#[test]
fn an_unknown_object_name_is_still_not_dispatched_on() {
    let Some(path) = built_library() else {
        println!("skipped: no built component found");
        return;
    };

    let library = unsafe { libloading::Library::new(&path) }.expect("the library loads");
    let get_class_object = unsafe { library.get::<GetClassObject>(b"GetClassObject") }
        .expect("GetClassObject is exported");
    let destroy_object = unsafe { library.get::<DestroyObject>(b"DestroyObject") }
        .expect("DestroyObject is exported");

    let name = wide("NoSuchObject");
    let mut component: *mut c_void = std::ptr::null_mut();

    let created = unsafe { get_class_object(name.as_ptr(), &mut component) };

    assert_ne!(created, 0, "GetClassObject stopped creating the object");

    // Destroy it, so this test does not leak the way a 1C session would not.
    unsafe { destroy_object(&mut component) };
}
