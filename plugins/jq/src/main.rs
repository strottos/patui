//! # Patui `jq` Plugin
//!
//! The `patui-jq` plugin is a standard plugin for the `patui` CLI. It provides support
//! for transforming JSON data similar to the `jq` command line tool.
//!
//! The following are some of the more popular commands in the `patui-std` plugin:
//! - transform: Asserts that a condition is true and fails the test if it is not.
//! - ReadFile: Reads a file from the filesystem that can be read by other steps.
//! - WriteFile: Writes a file to the filesystem with data from other steps.
//! - StaticData: Provides static data that can be used by other steps.

mod functions;

#[ptplugin::main(
    JqPluginServer,
    name = "jq",
    description = "Perform transforms on JSON data similar to the `jq` CLI tool.",
    r#type = "transform",
    build_struct = (functions::Transform),
)]
fn main() -> ptplugin::Result<()> {
    Ok(())
}
