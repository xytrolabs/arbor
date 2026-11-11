# Default Nushell Environment Config File
# These "sensible defaults" are set before the user's `env.nu` is loaded
#
# version = "0.117.0"

$env.PROMPT_COMMAND = {||
    let dir = match (do -i { $env.PWD | path relative-to $nu.home-dir }) {
        null => $env.PWD
        '' => '~'
        $relative_pwd => ([~ $relative_pwd] | path join)
    }

    let colors: record<path: string, separator: string, glyph: string> = match [(config use-colors), (is-admin)] {
        [false, _] => {path: '', separator: '', glyph: ''}
        [true, true] => {path: (ansi red_bold), separator: (ansi light_red_bold), glyph: (ansi red)}
        [true, false] => {path: (ansi green_bold), separator: (ansi light_green_bold), glyph: (ansi green)}
    }
    let path_segment = $"($colors.path)($dir)(ansi reset)"

    # Arbor: a tree glyph marks the prompt so the shell is identifiable at a
    # glance, while keeping the familiar Nushell path layout. A plain
    # double-quoted unicode escape is used so no special font is required.
    let glyph_segment = if (config use-colors) { $"($colors.glyph)\u{1F332}(ansi reset) " } else { '' }

    ($glyph_segment + ($path_segment | str replace --all (char path_sep) $"($colors.separator)(char path_sep)($colors.path)"))
}

$env.PROMPT_COMMAND_RIGHT = {||
    # create a right prompt in magenta with green separators and am/pm underlined
    let colors: record<date: string, separator: string, ampm: string, fail: string> = if (config use-colors) {
        {date: (ansi magenta), separator: (ansi green), ampm: (ansi magenta_underline), fail: (ansi red_bold)}
    } else {
        {date: '', separator: '', ampm: '', fail: ''}
    }
    let time_segment = ([
        (ansi reset)
        $colors.date
        (date now | format date '%x %X') # try to respect user's locale
    ] | str join | str replace --regex --all "([/:])" $"($colors.separator)${1}($colors.date)" |
        str replace --regex --all "([AP]M)" $"($colors.ampm)${1}")

    let last_exit_code = if ($env.LAST_EXIT_CODE != 0) {([
        $colors.fail
        $env.LAST_EXIT_CODE
    ] | str join)
    } else { "" }

    ([$last_exit_code, (char space), $time_segment] | str join)
}
