//! Proc macros for Bouldy runtime mods.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Fields, ItemStruct};

/// Generate the C ABI entrypoints for a Bouldy Rust mod.
///
/// The annotated struct must implement `Default` and `bouldy_runtime::Mod`.
/// For the MVP, this macro is intended for a single unit-like mod struct per
/// `cdylib`.
#[proc_macro_attribute]
pub fn unreal_mod(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;

    if !matches!(input.fields, Fields::Unit) {
        let expanded = quote! {
            #input
            compile_error!("#[unreal_mod] currently supports only unit-like structs");
        };
        return TokenStream::from(expanded);
    }

    let expanded = quote! {
        #input

        static BOULDY_MOD_INSTANCE: ::std::sync::Mutex<Option<#ident>> =
            ::std::sync::Mutex::new(None);

        fn bouldy_with_mod_instance<R>(f: impl FnOnce(&mut #ident) -> R) -> R {
            let mut guard = BOULDY_MOD_INSTANCE
                .lock()
                .unwrap_or_else(|err| err.into_inner());
            if guard.is_none() {
                *guard = Some(<#ident as ::std::default::Default>::default());
            }
            f(guard.as_mut().expect("Bouldy mod instance must exist"))
        }

        extern "C" fn bouldy_tick_trampoline(delta: f32) {
            ::bouldy_runtime::tick_registered_mod(delta, |delta| {
                bouldy_with_mod_instance(|module| {
                    <#ident as ::bouldy_runtime::Mod>::on_tick(module, delta);
                });
            });
        }

        extern "C" fn bouldy_shutdown_trampoline() {
            ::bouldy_runtime::shutdown_registered_mod(|| {
                bouldy_with_mod_instance(|module| {
                    <#ident as ::bouldy_runtime::Mod>::on_shutdown(module);
                });
            });
        }

        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn unreal_rust_init(api: *mut ::bouldy_runtime::UnrealApi) -> bool {
            ::bouldy_runtime::init_with_base_api(api, |ctx| {
                bouldy_with_mod_instance(|module| {
                    <#ident as ::bouldy_runtime::Mod>::on_init(module, ctx);
                });
            })
        }

        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn bouldy_rust_init_v1(api: *mut ::bouldy_runtime::UnrealApiV1) -> bool {
            ::bouldy_runtime::init_with_v1_api(
                api,
                bouldy_tick_trampoline,
                bouldy_shutdown_trampoline,
                |ctx| {
                    bouldy_with_mod_instance(|module| {
                        <#ident as ::bouldy_runtime::Mod>::on_init(module, ctx);
                    });
                },
            )
        }
    };

    TokenStream::from(expanded)
}
