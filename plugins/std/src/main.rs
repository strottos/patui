//! # Patui Standard Plugin
//!
//! The `patui-std` plugin is a standard plugin for the `patui` CLI. It provides a set of standard
//! commands that are useful for testing.
//!
//! The following are some of the more popular commands in the `patui-std` plugin:
//! - Assert: Asserts that a condition is true and fails the test if it is not.
//! - ReadFile: Reads a file from the filesystem that can be read by other steps.
//! - WriteFile: Writes a file to the filesystem with data from other steps.
//! - StaticData: Provides static data that can be used by other steps.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod functions;

#[ptplugin::main(StdPluginServer, build_struct = (
    functions::Assertion,
    functions::FileRead,
    functions::StaticData,
))]
fn main() -> ptplugin::Result<()> {
    Ok(())
}
