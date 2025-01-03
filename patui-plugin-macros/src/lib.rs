mod entry;

use proc_macro::TokenStream;

/// Setup the main function for the program
#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    entry::main(args.into(), item.into()).into()
}
