use arbor_plugin::{JsonSerializer, serve_plugin};
use arbor_plugin_query::Query;

fn main() {
    serve_plugin(&Query {}, JsonSerializer {})
}
