use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    self,
    parse::{self},
    parse_macro_input,
    punctuated::Punctuated,
    ItemStruct, LitStr,
};
use syn::{Ident, ImplItem, ItemImpl};

// let (path, value) = match attribute.parse_meta().unwrap() {
//     syn::Meta::NameValue(syn::MetaNameValue {
//         name,
//         lit: syn::Lit::Str(s),
//         ..
//     }) => (path, s.value()),
//     _ => panic!("malformed attribute syntax"),
// };

#[proc_macro_derive(AddInComponent)]
pub fn add_in_component(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemStruct);

    //     let mut field_values: TokenStream = TokenStream::new();

    //     match item.fields {
    //         syn::Fields::Named(fields) => {
    //             for field in &fields.named {
    //                 let field_name = field.ident.clone().unwrap();
    //                 match field_name {
    //                     _ if field_name == String::from("name") => {}
    //                     name => {
    //                         // syn::FieldValue::from(quote! {
    //                         //     #name: 1
    //                         // });
    //                         // field_values.extend(
    //                         //     quote! {
    //                         //         #name: 1
    //                         //     }
    //                         //     .into(),
    //                         // );

    //                         // properties.push(
    //                         // syn::Field::parse_named
    //                         //     .parse2(quote! { #name:  })
    //                         //     .unwrap(),
    //                         // ),
    //                     }
    //                 }
    //             }
    //         }
    //         _ => {}
    //     }

    //     let name = item.ident;
    //     let name_string = &name.to_string();

    //     // let properties: TokenStream = quote! { #properties }.into();

    //     // let ar = quote! {
    //     //     #field_values
    //     // };

    //     quote! {

    //         // impl #name {
    //         //     pub fn new() -> Self {
    //         //         Self {
    //         //             // name: String::from(#name_string),
    //         //             // name: String::from(""),
    //         //             // super_field1: 1,

    //         //         }
    //         //     }
    //         // }

    //         impl #name {
    //             pub fn super_fun() {

    //             }
    //         }

    //     }
    //     .into()

    let mut lang_names: HashMap<String, String> = HashMap::new();

    for attr in &item.attrs {
        if !attr.path().is_ident("component") {
            continue;
        }

        let nested = attr
            .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
            .unwrap();

        for meta in nested {
            match &meta {
                // #[procedure(ru(N))]
                syn::Meta::List(meta_value) => {
                    let lang = meta.path().get_ident().unwrap().to_string();
                    let method_name: Ident = meta_value.parse_args().unwrap();
                    let method_name = method_name.to_string();
                    lang_names.insert(lang, method_name);
                }
                _ => {
                    return syn::Error::new_spanned(meta, "unrecognized argument")
                        .to_compile_error()
                        .into();
                }
            }
        }
    }

    let name = item.ident;
    let component_name = lang_names.get("ru").unwrap();

    quote!(

        impl ones_addin::ffi::AddInBase for #name {

            fn init(&mut self, interface: &'static ones_addin::ffi::Connection, string: String) -> bool {
                self.data = string;
                self.connection = Some(interface);
                true
            }
            fn done(&mut self) {}

            fn register_extension_as(&mut self) -> &'static [u16] {
                &ones_addin::utf16::utf16_null!(#component_name)
            }

        }

        // The properties interface is answered with "no properties" and does not
        // read its arguments. Without this the unused parameters warn in every
        // crate that invokes the derive, which is not the invoking crate's fault.
        #[allow(unused_variables)]
        impl ones_addin::ffi::AddInProps for #name {

            fn get_n_props(&mut self) -> usize {
                0
            }
            fn find_prop(&mut self, name: &[u16]) -> Option<usize> {
                None
            }
            fn get_prop_name(&mut self, num: usize, _alias: usize) -> Option<&'static [u16]> {
                None
            }
            fn get_prop_val(&mut self, num: usize, val: ones_addin::ffi::ReturnValue) -> bool {
                val.set_empty();
                false
            }
            fn set_prop_val(&mut self, num: usize, val: &ones_addin::ffi::ParamValue) -> bool {
                false
            }
            fn is_prop_readable(&mut self, _num: usize) -> bool {
                true
            }
            fn is_prop_writable(&mut self, num: usize) -> bool {
                false
            }

        }

        impl ones_addin::ffi::AddInLang for #name {
            fn set_locale(&mut self, _loc: &[u16]) {}
            fn set_user_interface_language_code(&mut self, _lang: &[u16]) {}
        }

    )
    .into()
}

#[proc_macro_attribute]
pub fn component(_: TokenStream, input: TokenStream) -> TokenStream {
    // let _ = parse_macro_input!(args as parse::Nothing);
    input
}

// #[proc_macro_attribute]
// pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
//     println!("{}", args.to_string());
//     // let mut item_struct = parse_macro_input!(input as ItemStruct);
//     // let _ = parse_macro_input!(args as parse::Nothing);

//     // if let syn::Fields::Named(ref mut fields) = item_struct.fields {
//     //     fields.named.push(
//     //         syn::Field::parse_named
//     //             .parse2(quote! { pub a: String })
//     //             .unwrap(),
//     //     );
//     // }

//     // quote! {
//     //     #item_struct
//     // }
//     // .into()
//     TokenStream::new()
// }

/// Publish the annotated methods of an `impl` block to 1C.
///
/// Each method carries `#[function(ru("Имя"), en("Name"))]` or
/// `#[procedure(...)]`. Both names are optional but at least one is required, and
/// the platform looks a method up under either, so a script may call
/// `СоздатьQRКод` or `Generate` and reach the same code.
///
/// `#[function]` may also carry the bare flag `blob`, which says the method's
/// result is a block of bytes rather than a string:
///
/// ```ignore
/// #[function(ru("СоздатьQRКод"), en("Generate"), blob)]
/// fn generate(&mut self, connection: &Connection) -> Result<Vec<u8>, QrError>
/// ```
///
/// The generated code then calls `ret_value.set_blob`, and 1C sees
/// `ДвоичныеДанные`; without the flag the value is encoded as UTF-16 and 1C sees a
/// `Строка`. The two are not interchangeable, and the flag is explicit because the
/// macro cannot infer the choice from the tokens of the return type.
///
/// Arguments after the receiver must be `&Connection` followed by zero or more
/// `&ParamValue`; a method whose result is `Err` answers the platform with a bare
/// `false`, which 1C reports without any message, so a component that wants to
/// explain itself returns `Ok` and carries the failure in the value.
#[allow(unused_variables)]
#[proc_macro_attribute]
pub fn methods_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let item_impl = parse_macro_input!(input as ItemImpl);
    let _ = parse_macro_input!(args as parse::Nothing);

    let mut method_names_ru: Vec<Option<String>> = vec![];
    let mut method_names_en: Vec<Option<String>> = vec![];

    let mut method_args_count: Vec<usize> = vec![];
    let mut method_has_ret_val: Vec<bool> = vec![];
    // Whether the method hands the platform a block of bytes rather than a string.
    // Set by `#[function(..., blob)]`, and explicit rather than inferred because
    // the generated code sees only the method's tokens: it cannot look through the
    // `Result` to ask what the `Ok` type is, and the two ways of handing a value
    // over are not interchangeable — a PNG sent as a string arrives in 1C as
    // mojibake, and a string sent as bytes arrives as a line of binary data.
    let mut method_returns_blob: Vec<bool> = vec![];
    let mut method_fn_names: Vec<&Ident> = vec![];

    for item in &item_impl.items {
        match &item {
            ImplItem::Fn(fn_item) => {
                let mut is_method = false;
                let mut is_function = false;
                // Declared here rather than beside the name flags it is parsed
                // with, because it is read after the attribute loop, next to
                // `method_has_ret_val`.
                let mut returns_blob = false;

                for attr in &fn_item.attrs {
                    is_function = attr.path().is_ident("function");
                    if !attr.path().is_ident("procedure") && !is_function {
                        continue;
                    }

                    is_method = true;

                    let mut has_ru_name = false;
                    let mut has_en_name = false;

                    let nested = match attr
                        .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
                    {
                        Ok(v) => v,
                        Err(_) => {
                            return syn::Error::new_spanned(
                                attr,
                                "Cannot parse attribute arguments",
                            )
                            .to_compile_error()
                            .into();
                        }
                    };

                    for meta in nested {
                        match &meta {
                            // #[procedure(ru(N))]
                            syn::Meta::List(meta_value) => match meta.path() {
                                path if path.is_ident("ru") => {
                                    match meta_value.parse_args::<LitStr>() {
                                        Ok(v) => {
                                            has_ru_name = true;
                                            method_names_ru.push(Some(v.value()));
                                        }
                                        Err(_) => {
                                            return syn::Error::new_spanned(
                                                meta,
                                                "unrecognized argument",
                                            )
                                            .to_compile_error()
                                            .into();
                                        }
                                    };
                                }
                                path if path.is_ident("en") => {
                                    match meta_value.parse_args::<LitStr>() {
                                        Ok(v) => {
                                            has_en_name = true;
                                            method_names_en.push(Some(v.value()));
                                        }
                                        Err(_) => {
                                            return syn::Error::new_spanned(
                                                meta,
                                                "unrecognized argument",
                                            )
                                            .to_compile_error()
                                            .into();
                                        }
                                    };
                                }
                                _ => {
                                    return syn::Error::new_spanned(meta, "unrecognized argument")
                                        .to_compile_error()
                                        .into();
                                }
                            },
                            // #[function(ru(N), en(N), blob)] — the flag that says
                            // the method's result is binary data. A bare word in an
                            // attribute list arrives as a path, not as a list, so it
                            // needs its own arm.
                            syn::Meta::Path(path) if path.is_ident("blob") => {
                                returns_blob = true;
                            }
                            _ => {
                                return syn::Error::new_spanned(meta, "unrecognized argument")
                                    .to_compile_error()
                                    .into();
                            }
                        }
                    }

                    if !has_ru_name && !has_en_name {
                        return syn::Error::new_spanned(
                            attr,
                            "Attribute must inlude ru() or en() method name",
                        )
                        .to_compile_error()
                        .into();
                    } else if !has_ru_name {
                        method_names_ru.push(None);
                    } else if !has_en_name {
                        method_names_en.push(None);
                    }
                }

                if !is_method {
                    continue;
                }

                method_fn_names.push(&fn_item.sig.ident);
                let mut args_count = 0;

                let mut index = 0;
                for input in &fn_item.sig.inputs {
                    if index == 0 {
                        index += 1;
                        if let syn::FnArg::Typed(_) = input {
                            return syn::Error::new_spanned(input, "1C method must have reciever")
                                .to_compile_error()
                                .into();
                        }
                    } else if let syn::FnArg::Typed(pat) = input {
                        match &*pat.ty {
                            syn::Type::Reference(v) => {
                                if let Some(lifetime) = &v.lifetime {
                                    return syn::Error::new_spanned(
                                        input,
                                        "Method arguments does not need lifetime specifier",
                                    )
                                    .to_compile_error()
                                    .into();
                                }

                                if let Some(mutability) = &v.mutability {
                                    return syn::Error::new_spanned(
                                        input,
                                        "Method arguments does not need to be mutable",
                                    )
                                    .to_compile_error()
                                    .into();
                                }

                                match &*v.elem {
                                    syn::Type::Path(v) => {
                                        if let Some(segment) = v.path.segments.last() {
                                            if segment.ident == "Connection" {
                                                if index != 1 {
                                                    return syn::Error::new_spanned(
                                                        &segment.ident,
                                                        "Connection argument must be second",
                                                    )
                                                    .to_compile_error()
                                                    .into();
                                                }
                                            } else if segment.ident == "ParamValue" {
                                                args_count += 1;
                                            } else {
                                                return syn::Error::new_spanned(
                                                    &segment.ident,
                                                    "Incorrect argument type. Must be either Connection or ParamValue",
                                                )
                                                .to_compile_error()
                                                .into();
                                            }
                                        } else {
                                            return syn::Error::new_spanned(
                                                &v.path.segments,
                                                "Incorrect argument type. Must be either Connection or ParamValue",
                                            )
                                            .to_compile_error()
                                            .into();
                                        }
                                    }
                                    v => {
                                        return syn::Error::new_spanned(
                                            input,
                                            "Incorrect method argument provided",
                                        )
                                        .to_compile_error()
                                        .into();
                                    }
                                }
                            }
                            _ => {
                                return syn::Error::new_spanned(
                                    input,
                                    "All method arguments must be of reference type",
                                )
                                .to_compile_error()
                                .into();
                            }
                        }
                    }
                }

                match &fn_item.sig.output {
                    syn::ReturnType::Default => {
                        if is_function {
                            return syn::Error::new_spanned(
                                &fn_item.sig.output,
                                "Function must have return type",
                            )
                            .to_compile_error()
                            .into();
                        }
                    }
                    syn::ReturnType::Type(_, return_type) => {
                        if !is_function {
                            return syn::Error::new_spanned(
                                &fn_item.sig.output,
                                "Procedure must not have return type",
                            )
                            .to_compile_error()
                            .into();
                        }
                        let return_type = &**return_type;
                        // TODO: Validate return type and save Ok(type) value
                    }
                }

                method_args_count.push(args_count);

                if is_function {
                    method_has_ret_val.push(true);
                } else {
                    // A procedure has no result, so `blob` has nothing to describe.
                    // Saying so here beats emitting code that silently ignores it.
                    if returns_blob {
                        return syn::Error::new_spanned(
                            &fn_item.sig.ident,
                            "blob() describes a return value, so it needs #[function], not #[procedure]",
                        )
                        .to_compile_error()
                        .into();
                    }

                    method_has_ret_val.push(false);
                }

                method_returns_blob.push(returns_blob);
            }
            _ => {
                unreachable!()
            }
        }
    }

    let impl_type = item_impl.self_ty;

    let items = &item_impl.items;

    let method_count = method_fn_names.len();

    let method_index = (0..method_count).map(syn::Index::from);
    let method_index_1 = method_index.clone();
    let method_index_2 = method_index.clone();
    let method_index_3 = method_index.clone();
    let method_index_4 = method_index.clone();
    let method_index_5 = method_index.clone();

    let find_method_legs = (0..method_count).map(|index| {
        let mut names: Vec<&str> = vec![];
        if let Some(name) = &method_names_ru[index] {
            names.push(name);
        }
        if let Some(name) = &method_names_en[index] {
            names.push(name);
        }
        let leg = quote! {
            #( name == &ones_addin::utf16::utf16_null!(#names) )||*
        };
        leg
    });

    let get_method_name_legs = (0..method_count).map(|index| {
        let method_name_ru = &method_names_ru[index];
        let method_name_en = &method_names_en[index];
        // println!("NAME {:?}", method_name_ru);
        if method_name_ru.is_some() && method_name_en.is_some() {
            quote! {
                match alias {
                    0 => Some(&ones_addin::utf16::utf16_null!(#method_name_en)),
                    1 => Some(&ones_addin::utf16::utf16_null!(#method_name_ru)),
                    _ => None,
                }
            }
        } else if method_name_ru.is_some() {
            quote! {
                Some(&ones_addin::utf16::utf16_null!(#method_name_ru))
            }
        } else {
            quote! {
                Some(&ones_addin::utf16::utf16_null!(#method_name_en))
            }
        }
    });

    let methods_call_procs = method_fn_names.iter().enumerate().map(|(index, fn_name)| {
        if method_has_ret_val[index] {
            quote!(false)
        } else {
            let i = (0..method_args_count[index]).map(syn::Index::from);
            let args = if method_args_count[index] > 0 {
                quote!(connection, #( &params[#i], )*)
            } else {
                quote!(connection)
            };
            quote! {
                {
                    self.#fn_name(#args);
                    true
                }
            }
        }
    });

    let methods_call_func = method_fn_names.iter().enumerate().map(|(index, fn_name)| {
        if !method_has_ret_val[index] {
            return quote!(false);
        }

        let i = (0..method_args_count[index]).map(syn::Index::from);
        let args = if method_args_count[index] > 0 {
            quote!(connection, #( &params[#i], )*)
        } else {
            quote!(connection)
        };

        // How the value reaches the platform. A string becomes UTF-16 and is
        // presented to the script as a Строка; a block of bytes is copied into the
        // platform's own memory manager and presented as ДвоичныеДанные. The
        // platform owns the copy — `set_blob` allocates through the memory manager
        // it handed over at attach time — so nothing here has to outlive the call.
        let store = if method_returns_blob[index] {
            quote! {
                ret_value.set_blob(val.as_slice());
            }
        } else {
            quote! {
                let units: Vec<u16> = val.encode_utf16().collect();
                ret_value.set_str(units.as_slice());
            }
        };

        quote! {
            {
                let result = self.#fn_name(#args);
                match result {
                    Ok(val) => {
                        #store
                        true
                    },
                    Err(_) => {
                        false
                    }
                }
            }
        }
    });

    quote! {
        impl ones_addin::ffi::AddinMethods for #impl_type {

            fn get_n_methods(&mut self) -> usize {
                #method_count
            }
            fn find_method(&mut self, name: &[u16]) -> Option<usize> {
                match name {
                    #( _ if #find_method_legs => Some(#method_index), )*
                    _ => None,
                }
            }
            fn get_method_name(&mut self, num: usize, alias: usize) -> Option<&'static [u16]> {
                match num {
                    #(
                        #method_index_1 => {
                            #get_method_name_legs
                        },
                    )*
                    _ => {
                        None
                    },
                }
            }
            fn get_n_params(&mut self, num: usize) -> usize {
                match num {
                    #( #method_index_2 => #method_args_count, )*
                    _ => 0,
                }
            }
            fn get_param_def_value(
                &mut self,
                _method_num: usize,
                _param_num: usize,
                value: ones_addin::ffi::ReturnValue,
            ) -> bool {
                value.set_empty();
                true
            }
            fn has_ret_val(&mut self, num: usize) -> bool {
                match num {
                    #( #method_index_3 => #method_has_ret_val, )*
                    _ => false,
                }
            }
            fn call_as_proc(&mut self, num: usize, params: &[ones_addin::ffi::ParamValue], connection: &ones_addin::ffi::Connection) -> bool {
                match num {
                    #( #method_index_4 => #methods_call_procs, )*
                    _ => false,
                }
            }
            fn call_as_func(&mut self, num: usize, params: &[ones_addin::ffi::ParamValue], ret_value: ones_addin::ffi::ReturnValue, connection: &ones_addin::ffi::Connection) -> bool {
                match num {
                    #( #method_index_5 => #methods_call_func, )*
                    _ => false,
                }
            }
        }

        impl #impl_type {
            #( #items )*
        }

    }
    .into()
}

#[proc_macro_attribute]
pub fn procedure(_: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn function(_: TokenStream, input: TokenStream) -> TokenStream {
    input
}
