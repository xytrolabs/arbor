use crate::{
    command,
    config_files::{self, setup_config},
};
use log::trace;
#[cfg(feature = "plugin")]
use arbor_cli::read_plugin_file;
use arbor_cli::{EvaluateCommandsOpts, evaluate_commands, evaluate_file, evaluate_repl};
use arbor_config::ConfigFileKind;
use arbor_engine::exit::kill_jobs_and_exit;
use arbor_protocol::{
    PipelineData, ShellError,
    engine::{EngineState, Stack},
    report_shell_error,
};
use arbor_utils::perf;
use arbor_utils::time::Instant;

pub(crate) fn run_commands(
    engine_state: &mut EngineState,
    mut stack: Stack,
    parsed_cli: command::ParsedCli,
    use_color: bool,
    commands: &arbor_protocol::Spanned<String>,
    input: PipelineData,
) {
    trace!("run_commands");

    let command::ParsedCli {
        nu: parsed_nu_cli_args,
        args_to_script,
        ..
    } = parsed_cli;

    let start_time = arbor_utils::time::Instant::now();
    let create_scaffold = !engine_state.config_dirs.config_home.exists();

    // if the --no-config-file(-n) option is NOT passed, load the plugin file,
    // load the default env file or custom (depending on whether env was overridden),
    // and maybe a custom config file
    //
    // if the --no-config-file(-n) flag is passed, do not load plugin, env, or config files
    if parsed_nu_cli_args.no_config_file.is_none() {
        #[cfg(feature = "plugin")]
        read_plugin_file(
            engine_state,
            parsed_nu_cli_args.plugin_file.as_ref().map(|s| s.span),
        );

        perf!("read plugins", start_time, use_color);

        let start_time = Instant::now();
        // If we have an env file override *OR* we have a login shell parameter, read the env file
        if engine_state.config_dirs.env_file.is_override()
            || parsed_nu_cli_args.login_shell.is_some()
        {
            config_files::read_config_file(
                engine_state,
                &mut stack,
                ConfigFileKind::Env,
                create_scaffold,
                true,
                parsed_nu_cli_args.env_file.as_ref(),
            );
        } else {
            config_files::read_default_env_file(engine_state, &mut stack)
        }

        perf!("read env.nu", start_time, use_color);

        let start_time = Instant::now();

        // If we have a config file override *OR* we have a login shell parameter, read the config file
        if engine_state.config_dirs.config_file.is_override()
            || parsed_nu_cli_args.login_shell.is_some()
        {
            config_files::read_config_file(
                engine_state,
                &mut stack,
                ConfigFileKind::Config,
                create_scaffold,
                true,
                parsed_nu_cli_args.config_file.as_ref(),
            );
        }

        perf!("read config.nu", start_time, use_color);

        // If we have a login shell parameter, read the login file
        let start_time = Instant::now();
        if parsed_nu_cli_args.login_shell.is_some() {
            config_files::read_loginshell_file(engine_state, &mut stack, false);
        }

        perf!("read login.nu", start_time, use_color);
    }

    // Startup ends when the commands start running.
    engine_state.finish_startup();

    let start_time = Instant::now();
    let result = evaluate_commands(
        commands,
        engine_state,
        &mut stack,
        input,
        args_to_script,
        EvaluateCommandsOpts {
            table_mode: parsed_nu_cli_args.table_mode,
            error_style: parsed_nu_cli_args.error_style,
            no_newline: parsed_nu_cli_args.no_newline.is_some(),
        },
    );
    perf!("evaluate_commands", start_time, use_color);

    if let Err(err) = result {
        if let ShellError::Exit { code, .. } = &err {
            kill_jobs_and_exit(engine_state, *code)
        }
        report_shell_error(Some(&stack), engine_state, &err);
        kill_jobs_and_exit(engine_state, err.exit_code().unwrap_or(0));
    }
}

pub(crate) fn run_file(
    engine_state: &mut EngineState,
    mut stack: Stack,
    parsed_cli: command::ParsedCli,
    use_color: bool,
    input: PipelineData,
) {
    trace!("run_file");

    let command::ParsedCli {
        nu: parsed_nu_cli_args,
        script_name,
        args_to_script,
    } = parsed_cli;

    // if the --no-config-file(-n) option is NOT passed, load the plugin file,
    // load the default env file or custom (depending on whether env was overridden),
    // and maybe a custom config file
    //
    // if the --no-config-file(-n) flag is passed, do not load plugin, env, or config files
    if parsed_nu_cli_args.no_config_file.is_none() {
        let start_time = Instant::now();
        let create_scaffold = !engine_state.config_dirs.config_home.exists();
        #[cfg(feature = "plugin")]
        read_plugin_file(
            engine_state,
            parsed_nu_cli_args.plugin_file.as_ref().map(|s| s.span),
        );
        perf!("read plugins", start_time, use_color);

        let start_time = Instant::now();
        // only want to load config and env if relative argument is provided.
        if engine_state.config_dirs.env_file.is_override() {
            config_files::read_config_file(
                engine_state,
                &mut stack,
                ConfigFileKind::Env,
                create_scaffold,
                true,
                parsed_nu_cli_args.env_file.as_ref(),
            );
        } else {
            config_files::read_default_env_file(engine_state, &mut stack)
        }
        perf!("read env.nu", start_time, use_color);

        let start_time = Instant::now();
        if engine_state.config_dirs.config_file.is_override() {
            config_files::read_config_file(
                engine_state,
                &mut stack,
                ConfigFileKind::Config,
                create_scaffold,
                true,
                parsed_nu_cli_args.config_file.as_ref(),
            );
        }
        perf!("read config.nu", start_time, use_color);
    }

    // Startup ends when the script starts running.
    engine_state.finish_startup();

    // Apply --table-mode / -m CLI flag override before evaluating the file
    if let Some(ref t_mode) = parsed_nu_cli_args.table_mode
        && let Ok(s) = t_mode.coerce_str()
        && let Ok(mode) = s.parse()
    {
        let mut config = engine_state.get_config().as_ref().clone();
        config.table.mode = mode;
        engine_state.set_config(config);
    }

    let start_time = Instant::now();

    // Arbor: `.ind` / `.arbor` / `.waash` scripts are written in Indent, a
    // separate language. Hand them to the Indent runtime instead of the Nu
    // parser. Config has already been loaded above, so `$env.config.arbor`
    // is honoured.
    if !script_name.is_empty()
        && arbor_cli::indent::script_kind(std::path::Path::new(&script_name))
            == arbor_cli::indent::ScriptKind::Indent
        && engine_state.get_config().arbor.indent_scripts
    {
        let path = script_name.clone();
        let arbor_config = &engine_state.get_config().arbor;

        // Explicit config path wins, then PATH / known locations.
        let indent_bin = if arbor_config.indent_binary.is_empty() {
            arbor_cli::indent::find_indent_binary()
        } else {
            Some(std::path::PathBuf::from(&arbor_config.indent_binary))
        };

        let Some(indent_bin) = indent_bin else {
            eprintln!(
                "arbor: could not find the `indent` runtime (needed for {path}).\n\
                 Install Indent, or set $env.config.arbor.indent_binary."
            );
            kill_jobs_and_exit(engine_state, 127);
        };

        let lib_dir = arbor_cli::indent::find_helper_lib();
        let exit_code = match arbor_cli::indent::run_indent_file(
            &indent_bin,
            std::path::Path::new(&path),
            lib_dir.as_deref(),
        ) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("arbor: failed to run Indent script: {err}");
                kill_jobs_and_exit(engine_state, 1);
            }
        };
        kill_jobs_and_exit(engine_state, exit_code);
    }

    let result = evaluate_file(
        script_name,
        &args_to_script,
        engine_state,
        &mut stack,
        input,
    );
    perf!("evaluate_file", start_time, use_color);

    if let Err(err) = result {
        if let ShellError::Exit { code, .. } = &err {
            kill_jobs_and_exit(engine_state, *code)
        }
        report_shell_error(Some(&stack), engine_state, &err);
        kill_jobs_and_exit(engine_state, err.exit_code().unwrap_or(0));
    }
}

pub(crate) fn run_repl(
    engine_state: &mut EngineState,
    mut stack: Stack,
    parsed_nu_cli_args: command::NushellCliArgs,
) -> Result<(), miette::ErrReport> {
    trace!("run_repl");
    let start_time = arbor_utils::time::Instant::now();

    if parsed_nu_cli_args.no_config_file.is_none() {
        setup_config(
            engine_state,
            &mut stack,
            parsed_nu_cli_args.login_shell.is_some(),
        );
    }

    // Reload use_color from config in case it's different from the default value
    let use_color = engine_state
        .get_config()
        .use_ansi_coloring
        .get(engine_state);
    perf!("setup_config", start_time, use_color);

    // Apply --table-mode / -m CLI flag override before starting the REPL
    if let Some(ref t_mode) = parsed_nu_cli_args.table_mode
        && let Ok(s) = t_mode.coerce_str()
        && let Ok(mode) = s.parse()
    {
        let mut config = engine_state.get_config().as_ref().clone();
        config.table.mode = mode;
        engine_state.set_config(config);
    }

    let start_time = Instant::now();
    let ret_val = evaluate_repl(
        engine_state,
        stack,
        parsed_nu_cli_args.execute,
        parsed_nu_cli_args.no_std_lib,
    );
    perf!("evaluate_repl", start_time, use_color);

    ret_val
}

#[cfg(feature = "lsp")]
pub(crate) fn run_lsp(
    mut engine_state: EngineState,
    parsed_nu_cli_args: command::NushellCliArgs,
    use_color: bool,
    start_time: arbor_utils::time::Instant,
) -> Result<(), miette::ErrReport> {
    if parsed_nu_cli_args.no_config_file.is_none() {
        let mut stack = arbor_protocol::engine::Stack::new();
        config_files::setup_config(&mut engine_state, &mut stack, false);
    }

    engine_state.finish_startup();
    perf!("lsp starting", start_time, use_color);
    arbor_lsp::LanguageServer::initialize_stdio_connection(engine_state)?.serve_requests()
}
