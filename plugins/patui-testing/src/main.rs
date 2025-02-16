//! # Patui Testing Plugin
//!
//! This plugin has one purpose only, it is for Patui to test the plugin system works and responds
//! as it expects. We test for things like messages being sent out of order, messages being
//! delayed, etc etc. This plugin is not meant to be used for anything other than testing, do not
//! use this plugin for your Patui tests.

mod functions;

#[ptplugin::main(
    PatuiTestPlugin,
    name = "testing",
    description = "Patui testing plugin. Used by Patui to test its own plugin system.",
    r#type = "testing",
    build_struct = (functions::Echo, functions::UnorderedList),
)]
fn main() -> ptplugin::Result<()> {
    Ok(())
}
