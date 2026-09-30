use std::{
    ffi::{c_int, c_long, c_ulong, c_void},
    ptr::{self, NonNull},
    slice::{from_raw_parts, from_raw_parts_mut},
};

#[repr(C)]
#[derive(Debug)]
pub enum AttachType {
    CanAttachNotIsolated = 1,
    CanAttachIsolated,
    CanAttachAny,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Tm {
    pub sec: c_int,   // seconds after the minute - [0, 60] including leap second
    pub min: c_int,   // minutes after the hour - [0, 59]
    pub hour: c_int,  // hours since midnight - [0, 23]
    pub mday: c_int,  // day of the month - [1, 31]
    pub mon: c_int,   // months since January - [0, 11]
    pub year: c_int,  // years since 1900
    pub wday: c_int,  // days since Sunday - [0, 6]
    pub yday: c_int,  // days since January 1 - [0, 365]
    pub isdst: c_int, // daylight savings time flag

    #[cfg(target_family = "unix")]
    pub gmtoff: std::ffi::c_long, // seconds east of UTC
    #[cfg(target_family = "unix")]
    pub zone: std::ffi::c_char, // timezone abbreviation
}

pub struct ReturnValue<'a> {
    mem: &'a MemoryManager,
    variant: &'a mut TVariant,
    result: &'a mut bool,
}

#[allow(dead_code)]
impl<'a> ReturnValue<'a> {
    pub fn set_empty(self) {
        self.variant.vt = VariantType::EMPTY;
    }
    pub fn set_i32(self, val: i32) {
        self.variant.vt = VariantType::I4;
        self.variant.value.i32 = val;
    }

    pub fn set_bool(self, val: bool) {
        self.variant.vt = VariantType::BOOL;
        self.variant.value.bool = val;
    }

    pub fn set_f64(self, val: f64) {
        self.variant.vt = VariantType::R8;
        self.variant.value.f64 = val;
    }

    pub fn set_date(self, val: Tm) {
        self.variant.vt = VariantType::TM;
        self.variant.value.tm = val;
    }

    pub fn set_str(self, val: &[u16]) {
        let Some(ptr) = self.mem.alloc_str(val.len()) else {
            *self.result = false;
            return;
        };

        unsafe { ptr::copy_nonoverlapping(val.as_ptr(), ptr.as_ptr(), val.len()) };

        self.variant.vt = VariantType::PWSTR;
        self.variant.value.data_str.ptr = ptr.as_ptr();
        self.variant.value.data_str.len = val.len() as u32;
    }

    pub fn set_blob(self, val: &[u8]) {
        let Some(ptr) = self.mem.alloc_blob(val.len()) else {
            *self.result = false;
            return;
        };

        unsafe { ptr::copy_nonoverlapping(val.as_ptr(), ptr.as_ptr(), val.len()) };

        self.variant.vt = VariantType::BLOB;
        self.variant.value.data_blob.ptr = ptr.as_ptr();
        self.variant.value.data_blob.len = val.len() as u32;
    }

    pub fn alloc_str(self, len: usize) -> Option<&'a mut [u16]> {
        let Some(ptr) = self.mem.alloc_str(len) else {
            *self.result = false;
            return None;
        };

        unsafe { ptr::write_bytes(ptr.as_ptr(), 0, len) };

        self.variant.vt = VariantType::PWSTR;
        self.variant.value.data_str.ptr = ptr.as_ptr();
        self.variant.value.data_str.len = len as u32;

        Some(unsafe { from_raw_parts_mut(ptr.as_ptr(), len) })
    }

    pub fn alloc_blob(self, len: usize) -> Option<&'a mut [u8]> {
        let Some(ptr) = self.mem.alloc_blob(len) else {
            *self.result = false;
            return None;
        };

        unsafe { ptr::write_bytes(ptr.as_ptr(), 0, len) };

        self.variant.vt = VariantType::BLOB;
        self.variant.value.data_blob.ptr = ptr.as_ptr();
        self.variant.value.data_blob.len = len as u32;

        Some(unsafe { from_raw_parts_mut(ptr.as_ptr(), len) })
    }
}

pub enum ParamValue<'a> {
    Empty,
    Bool(bool),
    I32(i32),
    F64(f64),
    Date(Tm),
    Str(&'a [u16]),
    Blob(&'a [u8]),
}

impl<'a> From<&'a TVariant> for ParamValue<'a> {
    fn from(param: &'a TVariant) -> ParamValue<'a> {
        unsafe {
            match param.vt {
                VariantType::EMPTY => Self::Empty,
                VariantType::BOOL => Self::Bool(param.value.bool),
                VariantType::I4 => Self::I32(param.value.i32),
                VariantType::R8 => Self::F64(param.value.f64),
                VariantType::TM => Self::Date(param.value.tm),
                VariantType::PWSTR => Self::Str(from_raw_parts(
                    param.value.data_str.ptr,
                    param.value.data_str.len as usize,
                )),
                VariantType::BLOB => Self::Blob(from_raw_parts(
                    param.value.data_blob.ptr,
                    param.value.data_blob.len as usize,
                )),
                _ => Self::Empty,
            }
        }
    }
}

#[repr(u16)]
#[allow(dead_code)]
// These variants are the platform's `VTYPE_` constants transcribed, so they are read
// against the Native API documentation; renaming them to Rust's convention would
// break that. The discriminants are the ABI, the names are not.
#[allow(clippy::upper_case_acronyms)]
enum VariantType {
    EMPTY = 0,
    NULL,
    I2,        //int16_t
    I4,        //int32_t
    R4,        //float
    R8,        //double
    DATE,      //DATE (double)
    TM,        //struct tm
    PSTR,      //struct str    string
    INTERFACE, //struct iface
    ERROR,     //int32_t errCode
    BOOL,      //bool
    VARIANT,   //struct _tVariant *
    I1,        //int8_t
    UI1,       //uint8_t
    UI2,       //uint16_t
    UI4,       //uint32_t
    I8,        //int64_t
    UI8,       //uint64_t
    INT,       //int   Depends on architecture
    UINT,      //unsigned int  Depends on architecture
    HRESULT,   //long hRes
    PWSTR,     //struct wstr
    BLOB,      //means in struct str binary data contain
    CLSID,     //UUID

    ARRAY = 0x2000,
    UNDEFINED = 0xFFFF,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DataStr {
    pub ptr: *mut u16,
    pub len: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DataBlob {
    pub ptr: *mut u8,
    pub len: u32,
}

#[repr(C)]
union VariantValue {
    pub bool: bool,
    pub i32: i32,
    pub f64: f64,
    pub tm: Tm,
    pub data_str: DataStr,
    pub data_blob: DataBlob,
}

#[repr(C)]
struct TVariant {
    value: VariantValue,
    elements: u32, //Dimension for an one-dimensional array in pvarVal
    vt: VariantType,
}

pub trait AddInBase {
    fn init(&mut self, interface: &'static Connection, str: String) -> bool;

    /// default 2000, don't use version 1000, because static objects are created
    fn get_info(&mut self) -> u16 {
        2000
    }
    fn done(&mut self);
    fn register_extension_as(&mut self) -> &'static [u16];
}

pub trait AddInProps {
    fn get_n_props(&mut self) -> usize;
    fn find_prop(&mut self, name: &[u16]) -> Option<usize>;
    fn get_prop_name(&mut self, num: usize, alias: usize) -> Option<&'static [u16]>;
    fn get_prop_val(&mut self, num: usize, val: ReturnValue) -> bool;
    fn set_prop_val(&mut self, num: usize, val: &ParamValue) -> bool;
    fn is_prop_readable(&mut self, num: usize) -> bool;
    fn is_prop_writable(&mut self, num: usize) -> bool;
}

pub trait AddinMethods {
    fn get_n_methods(&mut self) -> usize;
    fn find_method(&mut self, name: &[u16]) -> Option<usize>;
    fn get_method_name(&mut self, num: usize, alias: usize) -> Option<&'static [u16]>;
    fn get_n_params(&mut self, num: usize) -> usize;
    fn get_param_def_value(
        &mut self,
        method_num: usize,
        param_num: usize,
        value: ReturnValue,
    ) -> bool;
    fn has_ret_val(&mut self, method_num: usize) -> bool;
    fn call_as_proc(
        &mut self,
        method_num: usize,
        params: &[ParamValue],
        connection: &Connection,
    ) -> bool;
    fn call_as_func(
        &mut self,
        method_num: usize,
        params: &[ParamValue],
        val: ReturnValue,
        connection: &Connection,
    ) -> bool;
}

pub trait AddInLang {
    fn set_locale(&mut self, loc: &[u16]);
    fn set_user_interface_language_code(&mut self, lang: &[u16]);
}

#[repr(C)]
struct This<const OFFSET: usize, T: AddInBase + AddInProps + AddinMethods + AddInLang> {
    ptr: *mut Component<T>,
}

// The lifetime is not elidable: this hands out a reference to the `Component` that
// contains the `This` being borrowed, so tying it to `&mut self` would be a
// different, and wrong, promise.
#[allow(clippy::needless_lifetimes)]
impl<'a, const OFFSET: usize, T: AddInBase + AddInProps + AddinMethods + AddInLang>
    This<OFFSET, T>
{
    unsafe fn get_component(&mut self) -> &'a mut Component<T> {
        let new_ptr = (self as *mut This<OFFSET, T> as *mut c_void)
            .sub(OFFSET * std::mem::size_of::<usize>());
        &mut *(new_ptr as *mut Component<T>)
    }
}

#[repr(C)]
struct InitDoneBaseVTable<T: AddInBase + AddInProps + AddinMethods + AddInLang> {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    init: unsafe extern "system" fn(&mut This<0, T>, *mut u8) -> bool, // &'static Connection) -> bool,
    set_mem_manager: unsafe extern "system" fn(&mut This<0, T>, &'static MemoryManager) -> bool,
    get_info: unsafe extern "system" fn(&mut This<0, T>) -> c_long,
    done: unsafe extern "system" fn(&mut This<0, T>),
}

unsafe extern "system" fn init<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<0, T>,
    interface: *mut u8, // &'static Connection,
) -> bool {
    let component = this.get_component();

    // let raw_pointer = component as *mut Component<T> as *mut *mut u8;
    // let slice = std::slice::from_raw_parts(raw_pointer, 10);

    let slice = unsafe { std::slice::from_raw_parts(interface, 128) };
    let string = format!("{:?}", slice);

    let wrapper = interface as *mut Connection;
    let wrapper = &mut *wrapper;
    let interface = wrapper; // &'static wrapper;

    component.connection = Some(interface);

    component.addin.init(interface, string)
}

unsafe extern "system" fn set_mem_manager<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<0, T>,
    mem: &'static MemoryManager,
) -> bool {
    let component = this.get_component();
    component.memory = Some(mem);
    true
}

unsafe extern "system" fn get_info<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<0, T>,
) -> c_long {
    let component = this.get_component();
    component.addin.get_info() as c_long
}

unsafe extern "system" fn done<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<0, T>,
) {
    let component = this.get_component();
    component.addin.done()
}

#[repr(C)]
struct LanguageExtenderBaseVTable<T: AddInBase + AddInProps + AddinMethods + AddInLang> {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    register_extension_as: unsafe extern "system" fn(&mut This<1, T>, *mut *mut u16) -> bool,
    get_n_props: unsafe extern "system" fn(&mut This<1, T>) -> c_long,
    find_prop: unsafe extern "system" fn(&mut This<1, T>, *const u16) -> c_long,
    get_prop_name: unsafe extern "system" fn(&mut This<1, T>, c_long, c_long) -> *const u16,
    get_prop_val: unsafe extern "system" fn(&mut This<1, T>, c_long, &mut TVariant) -> bool,
    set_prop_val: unsafe extern "system" fn(&mut This<1, T>, c_long, &TVariant) -> bool,
    is_prop_readable: unsafe extern "system" fn(&mut This<1, T>, c_long) -> bool,
    is_prop_writable: unsafe extern "system" fn(&mut This<1, T>, c_long) -> bool,
    get_n_methods: unsafe extern "system" fn(&mut This<1, T>) -> c_long,
    find_method: unsafe extern "system" fn(&mut This<1, T>, *const u16) -> c_long,
    get_method_name: unsafe extern "system" fn(&mut This<1, T>, c_long, c_long) -> *const u16,
    get_n_params: unsafe extern "system" fn(&mut This<1, T>, c_long) -> c_long,
    get_param_def_value:
        unsafe extern "system" fn(&mut This<1, T>, c_long, c_long, &mut TVariant) -> bool,
    has_ret_val: unsafe extern "system" fn(&mut This<1, T>, c_long) -> bool,
    call_as_proc:
        unsafe extern "system" fn(&mut This<1, T>, c_long, *const TVariant, c_long) -> bool,
    call_as_func: unsafe extern "system" fn(
        &mut This<1, T>,
        c_long,
        &mut TVariant,
        *const TVariant,
        c_long,
    ) -> bool,
}

unsafe extern "system" fn register_extension_as<
    T: AddInBase + AddInProps + AddinMethods + AddInLang,
>(
    this: &mut This<1, T>,
    name: *mut *mut u16,
) -> bool {
    let component = this.get_component();
    let Some(allocator) = component.memory else {
        return false;
    };

    let extension_name = component.addin.register_extension_as();

    let Some(ptr) = allocator.alloc_str(extension_name.len()) else {
        return false;
    };
    ptr::copy_nonoverlapping(extension_name.as_ptr(), ptr.as_ptr(), extension_name.len());
    *name = ptr.as_ptr();

    true
}

unsafe extern "system" fn get_n_props<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
) -> c_long {
    let component = this.get_component();
    component.addin.get_n_props() as c_long
}

unsafe extern "system" fn find_prop<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    name: *const u16,
) -> c_long {
    let component = this.get_component();
    let name = get_str(name);
    match component.addin.find_prop(name) {
        Some(i) => i as c_long,
        None => -1,
    }
}

unsafe extern "system" fn get_prop_name<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    num: c_long,
    alias: c_long,
) -> *const u16 {
    let component = this.get_component();
    let Some(allocator) = component.memory else {
        return ptr::null();
    };
    let Some(prop_name) = component.addin.get_prop_name(num as usize, alias as usize) else {
        return ptr::null();
    };
    let Some(ptr) = allocator.alloc_str(prop_name.len()) else {
        return ptr::null();
    };
    ptr::copy_nonoverlapping(prop_name.as_ptr(), ptr.as_ptr(), prop_name.len());

    ptr.as_ptr()
}

unsafe extern "system" fn get_prop_val<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    component: &mut This<1, T>,
    num: c_long,
    val: &mut TVariant,
) -> bool {
    let component = component.get_component();
    let Some(mem) = component.memory else {
        return false;
    };

    let mut result = true;
    let return_value = ReturnValue {
        mem,
        variant: val,
        result: &mut result,
    };
    component.addin.get_prop_val(num as usize, return_value) && result
}

unsafe extern "system" fn set_prop_val<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    num: c_long,
    val: &TVariant,
) -> bool {
    let component = this.get_component();
    let param = ParamValue::from(val);
    component.addin.set_prop_val(num as usize, &param)
}

unsafe extern "system" fn is_prop_readable<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    num: c_long,
) -> bool {
    let component = this.get_component();
    component.addin.is_prop_readable(num as usize)
}

unsafe extern "system" fn is_prop_writable<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    num: c_long,
) -> bool {
    let component = this.get_component();
    component.addin.is_prop_writable(num as usize)
}

unsafe extern "system" fn get_n_methods<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
) -> c_long {
    let component = this.get_component();
    component.addin.get_n_methods() as c_long
}

unsafe extern "system" fn find_method<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    name: *const u16,
) -> c_long {
    let component = this.get_component();
    let name = get_str(name);
    match component.addin.find_method(name) {
        Some(i) => i as c_long,
        None => -1,
    }
}

unsafe extern "system" fn get_method_name<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    num: c_long,
    alias: c_long,
) -> *const u16 {
    let component = this.get_component();
    let Some(allocator) = component.memory else {
        return ptr::null();
    };
    let Some(method_name) = component
        .addin
        .get_method_name(num as usize, alias as usize)
    else {
        return ptr::null();
    };
    let Some(ptr) = allocator.alloc_str(method_name.len()) else {
        return ptr::null();
    };

    ptr::copy_nonoverlapping(method_name.as_ptr(), ptr.as_ptr(), method_name.len());

    ptr.as_ptr()
}

unsafe extern "system" fn get_n_params<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    num: c_long,
) -> c_long {
    let component = this.get_component();
    component.addin.get_n_params(num as usize) as c_long
}

unsafe extern "system" fn get_param_def_value<
    T: AddInBase + AddInProps + AddinMethods + AddInLang,
>(
    this: &mut This<1, T>,
    method_num: c_long,
    param_num: c_long,
    val: &mut TVariant,
) -> bool {
    let component = this.get_component();
    let Some(mem) = component.memory else {
        return false;
    };

    let mut result = true;
    let return_value = ReturnValue {
        mem,
        variant: val,
        result: &mut result,
    };

    component
        .addin
        .get_param_def_value(method_num as usize, param_num as usize, return_value)
        && result
}

unsafe extern "system" fn has_ret_val<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    method_num: c_long,
) -> bool {
    let component = this.get_component();
    component.addin.has_ret_val(method_num as usize)
}

unsafe extern "system" fn call_as_proc<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    method_num: c_long,
    params: *const TVariant,
    size_array: c_long,
) -> bool {
    let component = this.get_component();
    let param_values = from_raw_parts(params, size_array as usize)
        .iter()
        .map(ParamValue::from)
        .collect::<Vec<ParamValue>>();

    let connection = component.connection.unwrap();

    component
        .addin
        .call_as_proc(method_num as usize, param_values.as_slice(), connection)
}

unsafe extern "system" fn call_as_func<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<1, T>,
    method_num: c_long,
    ret_value: &mut TVariant,
    params: *const TVariant,
    size_array: c_long,
) -> bool {
    let component = this.get_component();
    let Some(mem) = component.memory else {
        return false;
    };

    let mut result = true;
    let return_value = ReturnValue {
        mem,
        variant: ret_value,
        result: &mut result,
    };

    let param_values = from_raw_parts(params, size_array as usize)
        .iter()
        .map(ParamValue::from)
        .collect::<Vec<ParamValue>>();

    let connection = component.connection.unwrap();

    component.addin.call_as_func(
        method_num as usize,
        param_values.as_slice(),
        return_value,
        connection,
    ) && result
}

#[repr(C)]
struct LocaleBaseVTable<T: AddInBase + AddInProps + AddinMethods + AddInLang> {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    set_locale: unsafe extern "system" fn(&mut This<2, T>, *const u16),
}

unsafe extern "system" fn set_locale<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    this: &mut This<2, T>,
    loc: *const u16,
) {
    let component = this.get_component();
    let loc = get_str(loc);
    component.addin.set_locale(loc)
}

#[repr(C)]
struct UserLanguageBaseVTable<T: AddInBase + AddInProps + AddinMethods + AddInLang> {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    set_user_interface_language_code: unsafe extern "system" fn(&mut This<3, T>, *const u16),
}

unsafe extern "system" fn set_user_interface_language_code<
    T: AddInBase + AddInProps + AddinMethods + AddInLang,
>(
    this: &mut This<3, T>,
    lang: *const u16,
) {
    let component = this.get_component();
    let lang = get_str(lang);
    component.addin.set_user_interface_language_code(lang)
}

#[repr(C)]
struct Component<T: AddInBase + AddInProps + AddinMethods + AddInLang> {
    vptr1: Box<InitDoneBaseVTable<T>>,
    vptr2: Box<LanguageExtenderBaseVTable<T>>,
    vptr3: Box<LocaleBaseVTable<T>>,
    vptr4: Box<UserLanguageBaseVTable<T>>,
    destroy: unsafe extern "system" fn(*mut *mut Component<T>),
    memory: Option<&'static MemoryManager>,
    connection: Option<&'static Connection>,
    addin: T,
}

unsafe extern "system" fn destroy<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    component: *mut *mut Component<T>,
) {
    let comp = Box::from_raw(*component);
    drop(comp);
}

/// Hand the platform an object built around `addin`.
///
/// # Safety
///
/// `component` must be a writable `*mut *mut c_void` owned by the caller — in
/// practice the out-parameter of `GetClassObject`. The pointer it receives must be
/// released with [`destroy_component`] exactly once.
pub unsafe fn create_component<T: AddInBase + AddInProps + AddinMethods + AddInLang>(
    component: *mut *mut c_void,
    addin: T,
) -> c_long {
    let vptr1 = Box::new(InitDoneBaseVTable {
        dtor: 0,
        #[cfg(target_family = "unix")]
        dtor2: 0,
        init,
        set_mem_manager,
        get_info,
        done,
    });

    let vptr2 = Box::new(LanguageExtenderBaseVTable {
        dtor: 0,
        #[cfg(target_family = "unix")]
        dtor2: 0,
        register_extension_as,
        get_n_props,
        find_prop,
        get_prop_name,
        get_prop_val,
        set_prop_val,
        is_prop_readable,
        is_prop_writable,
        get_n_methods,
        find_method,
        get_method_name,
        get_n_params,
        get_param_def_value,
        has_ret_val,
        call_as_proc,
        call_as_func,
    });

    let vptr3 = Box::new(LocaleBaseVTable {
        dtor: 0,
        #[cfg(target_family = "unix")]
        dtor2: 0,
        set_locale,
    });

    let vptr4 = Box::new(UserLanguageBaseVTable {
        dtor: 0,
        #[cfg(target_family = "unix")]
        dtor2: 0,
        set_user_interface_language_code,
    });

    let c = Box::new(Component {
        vptr1,
        vptr2,
        vptr3,
        vptr4,
        destroy: destroy::<T>,
        memory: None,
        connection: None,
        addin,
    });

    *component = Box::into_raw(c) as *mut c_void;
    1
}

/// Release an object that [`create_component`] created.
///
/// # Safety
///
/// `component` must hold a pointer this library returned from `create_component`,
/// still live, and never passed here before.
pub unsafe fn destroy_component(component: *mut *mut c_void) -> c_long {
    #[repr(C)]
    struct ComponentWrapper {
        vptr1: usize,
        vptr2: usize,
        vptr3: usize,
        vptr4: usize,
        destroy: unsafe extern "system" fn(*mut *mut c_void),
    }

    let wrapper = *component as *mut ComponentWrapper;
    let wrapper = &mut *wrapper;
    (wrapper.destroy)(component);
    *component = ptr::null_mut();
    0
}

#[repr(C)]
struct MemoryManagerVTable {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    alloc_memory: unsafe extern "system" fn(&MemoryManager, *mut *mut c_void, c_ulong) -> bool,
    free_memory: unsafe extern "system" fn(&MemoryManager, *mut *mut c_void),
}

#[repr(C)]
struct MemoryManager {
    vptr: &'static MemoryManagerVTable,
}

impl MemoryManager {
    pub fn alloc_blob(&self, size: usize) -> Option<NonNull<u8>> {
        let mut ptr = ptr::null_mut::<c_void>();
        unsafe {
            if (self.vptr.alloc_memory)(self, &mut ptr, size as c_ulong) {
                NonNull::new(ptr as *mut u8)
            } else {
                None
            }
        }
    }

    pub fn alloc_str(&self, size: usize) -> Option<NonNull<u16>> {
        let mut ptr = ptr::null_mut::<c_void>();
        unsafe {
            if (self.vptr.alloc_memory)(self, &mut ptr, size as c_ulong * 2) {
                NonNull::new(ptr as *mut u16)
            } else {
                None
            }
        }
    }
}

// WCHAR_T*       -> *mut u16
// const WCHAR_T* -> *const u16
// tVariant*      -> *const TVariant
// long *         -> *const c_long
// WCHAR_T**      -> *mut *mut u16

#[repr(C)]
struct IAddInDefBaseVTable {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    add_error: unsafe extern "system" fn(&Connection, u8, *const u16, *const u16, c_long) -> bool,
    read: unsafe extern "system" fn(
        &Connection,
        *mut u16,
        *const TVariant,
        *const c_long,
        *mut *mut u16,
    ) -> bool,
    write: unsafe extern "system" fn(&Connection, *mut u16, *const TVariant) -> bool,
    register_profile_as: unsafe extern "system" fn(&Connection, *mut u16) -> bool,
    set_event_buffer_depth: unsafe extern "system" fn(&Connection, c_long) -> bool,
    get_event_buffer_depth: unsafe extern "system" fn(&Connection) -> c_long,
    external_event: unsafe extern "system" fn(&Connection, *mut u16, *mut u16, *mut u16) -> bool,
    // `void` on the platform's side of the ABI, so no return type here: declaring
    // `-> c_void` promises a return value that the C++ slot never produces. Caught
    // by the `c_void`-return lint that arrived with 1.101-nightly.
    clean_event_buffer: unsafe extern "system" fn(&Connection),
    set_status_line: unsafe extern "system" fn(&Connection, *mut u16) -> bool,
    reset_status_line: unsafe extern "system" fn(&Connection),
}

#[repr(C)]
struct IAddInDefBaseExVTable {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    get_interface:
        unsafe extern "system" fn(&Connection, iface: InterfaceTypes) -> &'static IMsgBox,
}

#[repr(C)]
pub struct Connection {
    vptr1: &'static IAddInDefBaseVTable,
    destroy: unsafe extern "system" fn(*mut *mut Connection),
    vptr2: &'static IAddInDefBaseExVTable,
}

#[allow(unused)]
#[repr(C)]
pub enum ErrorType {
    None = 1000,
    Ordinary = 1001,
    Attention = 1002,
    Important = 1003,
    VeryImportant = 1004,
    Info = 1005,
    Fail = 1006,
    MsgboxAttention = 1007,
    MsgboxInfo = 1008,
    MsgboxFail = 1009,
}

const BASE_ERRNO: i64 = 0x7; // Replace with the desired value

// `c_long` is 64-bit on Linux and 32-bit on Windows, so the conversions below are
// necessary on one of the two targets this component is built for and redundant on
// the other. The platform reads the result as a 32-bit HRESULT either way.
#[allow(clippy::unnecessary_cast)]
fn result_from_errno(x: i64) -> c_long {
    if x <= 0 {
        x as c_long
    } else {
        ((x & 0x0000FFFF) | (BASE_ERRNO << 16) | 0x80000000) as c_long
    }
}

#[repr(C)]
pub enum InterfaceTypes {
    MsgBox = 0,
    PlatformInfo,
    // `target_family` is `unix` on Android, so the vendored `target_family =
    // "android"` never matched and this variant shifted the ones after it on the
    // one platform that does have it. Inert on the two targets this project
    // builds for.
    #[cfg(target_os = "android")]
    AndroidComponentHelper,
    AttachedInfo,
}

#[repr(C)]
struct IAttachedInfoVTable {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    get_attached_info: unsafe extern "system" fn(&IAttachedInfo) -> c_int,
}

#[repr(C)]
pub struct IAttachedInfo {
    vptr1: usize,
    vptr2: &'static IAttachedInfoVTable,
}

#[repr(C)]
#[derive(Debug)]
struct IMsgBoxVTable {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    confirm: unsafe extern "system" fn(&IMsgBox, *const u16, *const TVariant) -> bool,
    alert: unsafe extern "system" fn(&IMsgBox, *const u16) -> bool,
}

#[repr(C)]
pub struct IInterface {
    pub _address: u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct IMsgBox {
    vptr1: &'static IMsgBoxVTable,
}

impl IMsgBox {
    pub fn alert(&self, message: &str) -> bool {
        // The buffer has to outlive the call: taking `.as_ptr()` straight off the
        // `collect()` leaves the platform a pointer into freed memory. The
        // vendored version did exactly that.
        let message: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
        unsafe { (self.vptr1.alert)(self, message.as_ptr()) }
    }
}

#[repr(C)]
struct IPlatformInfoVTable {
    dtor: usize,
    #[cfg(target_family = "unix")]
    dtor2: usize,
    get_platform_info: unsafe extern "system" fn(&IPlatformInfo) -> *mut *mut c_void,
}

#[repr(C)]
pub struct IPlatformInfo {
    vptr1: usize,
    vptr2: &'static IPlatformInfoVTable,
}

pub enum Interface {
    AttachedInfo(&'static IAttachedInfo),
    MsgBox(&'static IMsgBox),
    PlatformInfo(&'static IPlatformInfo),
    None,
}

impl Connection {
    // Adds the error message
    // /**
    //  *  @param wcode - error code
    //  *  @param source - source of error
    //  *  @param descr - description of error
    //  *  @param scode - error code (HRESULT)
    //  *  @return the result of
    //  */
    // virtual bool ADDIN_API AddError(unsigned short wcode, const WCHAR_T* source,
    //     const WCHAR_T* descr, long scode) = 0;
    pub fn add_error(&self, error_type: ErrorType, source: &str, descr: &str, scode: i64) -> bool {
        // Both buffers must outlive the call; see `IMsgBox::alert`.
        let source: Vec<u16> = source.encode_utf16().chain(Some(0)).collect();
        let descr: Vec<u16> = descr.encode_utf16().chain(Some(0)).collect();
        let scode = result_from_errno(scode);

        unsafe {
            (self.vptr1.add_error)(
                self,
                error_type as u8,
                source.as_ptr(),
                descr.as_ptr(),
                scode,
            )
        }
    }

    pub fn get_component(&self, interface_type: InterfaceTypes) -> Interface {
        let i_type = match &interface_type {
            InterfaceTypes::AttachedInfo => InterfaceTypes::AttachedInfo,
            InterfaceTypes::MsgBox => InterfaceTypes::MsgBox,
            InterfaceTypes::PlatformInfo => InterfaceTypes::PlatformInfo,
        };
        unsafe {
            let intrf_ptr = (self.vptr2.get_interface)(self, interface_type);

            // let intrf_ptr = (self.vptr2.get_interface)(&self, interface_type);
            match i_type {
                InterfaceTypes::AttachedInfo => {
                    // let wrapper = intrf_ptr as *mut IAttachedInfo;
                    // let wrapper = &mut *wrapper;
                    Interface::None
                }
                InterfaceTypes::MsgBox => {
                    // let wrapper = intrf_ptr as *mut IMsgBox;
                    // let wrapper = &mut *wrapper;
                    // Interface::MsgBox(wrapper)
                    Interface::MsgBox(intrf_ptr)
                }
                InterfaceTypes::PlatformInfo => {
                    // let wrapper = intrf_ptr as *mut IPlatformInfo;
                    // let wrapper = &mut *wrapper;
                    Interface::None
                }
            }
        }
    }
}

unsafe fn get_str<'a>(s: *const u16) -> &'a [u16] {
    unsafe fn strlen(s: *const u16) -> usize {
        let mut i = 0;
        while *s.add(i) != 0 {
            i += 1;
        }
        i + 1
    }

    let len = strlen(s);
    from_raw_parts(s, len)
}
