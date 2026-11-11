mod contains;

use crate::PolarsPlugin;
use arbor_plugin::PluginCommand;

pub use contains::ListContains;

pub(crate) fn list_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![Box::new(ListContains)]
}
