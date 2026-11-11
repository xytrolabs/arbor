use arbor_plugin::{MsgPackSerializer, serve_plugin};
use arbor_plugin_gstat::GStatPlugin;

fn main() {
    serve_plugin(&GStatPlugin, MsgPackSerializer {})
}
