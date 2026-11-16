//! Arbor-specific shell settings (`$env.config.arbor`).
//!
//! These options drive features inherited from Waash: Ctrl+B
//! move-to-background, Indent script support and multi-line bash block
//! delegation.

use super::prelude::*;
use crate as arbor_protocol;

#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArborConfig {
    /// Key that moves the running foreground command to the background.
    ///
    /// Accepts `Ctrl+<a-z>`. Defaults to `Ctrl+B`. An unrecognised value
    /// falls back to the default rather than disabling the feature.
    pub bg_shortcut: String,

    /// Run `.ind` / `.arbor` scripts with the Indent runtime.
    pub indent_scripts: bool,

    /// Explicit path to the `indent` executable.
    ///
    /// An empty string means "search `PATH`, then `~/.local/bin`, then the
    /// upstream installer location".
    pub indent_binary: String,

    /// Collect multi-line bash control-flow blocks (`if`/`for`/`while`/`case`,
    /// function definitions, `{ ... }`) typed at the prompt and run them
    /// through `bash`.
    pub bash_blocks: bool,

    /// Show the git branch / dirty / ahead-behind segment in the prompt.
    ///
    /// The prompt is rebuilt on every render, and looking up git state spawns
    /// an external `git` process (~20 ms). That is invisible for interactive
    /// use but adds up when many commands are run in a burst, such as during
    /// scripted or agent-driven work. Set this to `false` in such setups.
    pub prompt_git: bool,
}

impl Default for ArborConfig {
    fn default() -> Self {
        Self {
            bg_shortcut: "Ctrl+B".to_string(),
            indent_scripts: true,
            indent_binary: String::new(),
            bash_blocks: true,
            prompt_git: true,
        }
    }
}

impl UpdateFromValue for ArborConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "bg_shortcut" => self.bg_shortcut.update(val, path, errors),
                "indent_scripts" => self.indent_scripts.update(val, path, errors),
                "indent_binary" => self.indent_binary.update(val, path, errors),
                "bash_blocks" => self.bash_blocks.update(val, path, errors),
                "prompt_git" => self.prompt_git.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
