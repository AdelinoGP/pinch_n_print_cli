use proc_macro::TokenStream;

/// Placeholder for the `#[slicer_module]` attribute macro.
#[proc_macro_attribute]
pub fn slicer_module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Placeholder for the `#[module_test]` attribute macro.
#[proc_macro_attribute]
pub fn module_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
