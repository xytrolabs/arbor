use arbor_plugin::{JsonSerializer, serve_plugin};
use arbor_plugin_inc::IncPlugin;

fn main() {
    serve_plugin(&IncPlugin, JsonSerializer {})
}
