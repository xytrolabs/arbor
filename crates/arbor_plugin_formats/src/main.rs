use arbor_plugin::{MsgPackSerializer, serve_plugin};
use arbor_plugin_formats::FormatCmdsPlugin;

fn main() {
    serve_plugin(&FormatCmdsPlugin, MsgPackSerializer {})
}
