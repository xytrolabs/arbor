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

    let use_colors = (config use-colors)
    let is_admin = (is-admin)

    # Arbor identity: a tree glyph plus a two-line user@host + path prompt.
    # The glyph is a plain BMP character, so no special font is required, and
    # it is omitted entirely when colours are off.
    let glyph_segment = if $use_colors {
        if $is_admin { $"(ansi red)\u{1F332}(ansi reset) " } else { $"(ansi green)\u{1F332}(ansi reset) " }
    } else { '' }

    let user_host = (do -i {
        let u = ($env.USER? | default (whoami))
        let h = (sys host | get hostname | str trim)
        if $use_colors {
            let c = if $is_admin { (ansi red_bold) } else { (ansi green_bold) }
            $"($c)($u)(ansi reset)(ansi dimmed)@(ansi reset)($c)($h)(ansi reset)"
        } else {
            $"($u)@($h)"
        }
    } | default '')

    let dir_segment = if $use_colors {
        let c = if $is_admin { (ansi red_bold) } else { (ansi green_bold) }
        let s = if $is_admin { (ansi light_red_bold) } else { (ansi light_green_bold) }
        $"($c)($dir)(ansi reset)" | str replace --all (char path_sep) $"($s)(char path_sep)($c)"
    } else {
        $dir
    }

    # git branch, dirty marker and ahead/behind counts.
    #
    # This runs on EVERY prompt render, so it must be cheap. A single
    # `git status --porcelain=v2 --branch` reports the branch name, the
    # ahead/behind counts and the dirty state in one process; the obvious
    # alternative (rev-parse + status + two rev-list calls) spawned four
    # processes per render, which cost ~80ms per prompt and made bursts of
    # commands feel sluggish.
    #
    # `complete` is used so git's stderr never leaks onto the terminal (a repo
    # without an upstream otherwise prints "fatal: no upstream configured").
    #
    # Set `$env.config.arbor.prompt_git = false` to skip this entirely: spawning
    # git costs ~20ms per render, which is invisible interactively but adds up
    # across large bursts of commands (scripted or agent-driven work).
    let use_git = ($env.config.arbor?.prompt_git? | default true)
    let git_segment = (if not $use_git { '' } else { do -i {
        let r = (git status --porcelain=v2 --branch | complete)
        if $r.exit_code != 0 {
            ''
        } else {
            let lines = ($r.stdout | lines)
            let head_line = ($lines | where {|l| $l | str starts-with '# branch.head ' } | first | default '')
            let branch = (if ($head_line | is-empty) { '' } else { $head_line | str substring 14.. | str trim })
            if ($branch | is-empty) or ($branch == '(detached)') {
                ''
            } else {
                # Any non-header line means the work tree is dirty.
                let dirty = (if ($lines | where {|l| not ($l | str starts-with '#') } | is-empty) { '' } else { '*' })
                let ab_line = ($lines | where {|l| $l | str starts-with '# branch.ab ' } | first | default '')
                let counts = (if ($ab_line | is-empty) {
                    ''
                } else {
                    # Format: "# branch.ab +<ahead> -<behind>"
                    let parts = ($ab_line | str replace '# branch.ab ' '' | split row ' ')
                    let ahead = ($parts | where {|p| $p | str starts-with '+' } | first | default '+0' | str substring 1.. | into int)
                    let behind = ($parts | where {|p| $p | str starts-with '-' } | first | default '-0' | str substring 1.. | into int)
                    if $ahead > 0 and $behind > 0 { $" +($ahead) -($behind)" } else if $ahead > 0 { $" +($ahead)" } else if $behind > 0 { $" -($behind)" } else { '' }
                })
                if $use_colors {
                    let c = if ($dirty | is-empty) { (ansi purple) } else { (ansi yellow_bold) }
                    $" ($c)\(($branch)($dirty)($counts)\)(ansi reset)"
                } else {
                    $" \(($branch)($dirty)($counts)\)"
                }
            }
        }
    } | default '' })

    # Exit code + duration on the second line, only when relevant.
    let status_segment = (do -i {
        let code = ($env.LAST_EXIT_CODE? | default 0)
        let dur = ($env.CMD_DURATION_MS? | default 0 | into int)
        mut parts = []
        if $code != 0 {
            $parts = ($parts | append $"(ansi red_bold)($code)(ansi reset)")
        }
        if $dur >= 1000 {
            let secs = ($dur / 1000 | math round --precision 1)
            $parts = ($parts | append $"(ansi yellow)($secs)s(ansi reset)")
        }
        if ($parts | is-empty) { '' } else { ($parts | str join " ") + " " }
    } | default '')

    $"($glyph_segment)($user_host)  ($dir_segment)($git_segment)(char nl)($status_segment)($glyph_segment)"
}

$env.PROMPT_COMMAND_RIGHT = {||
    if not (config use-colors) { return '' }

    # Right-aligned status line: load average, background jobs, wall clock.
    let battery_segment = (do -i {
        # Best-effort battery read; absent on desktops and most VMs.
        let cap = (open /sys/class/power_supply/BAT0/capacity 2>/dev/null | str trim)
        if ($cap | is-empty) { '' } else {
            let icon = if ($cap | into int) <= 20 { "\u{1F7E1}" } else { "\u{1F50B}" }
            $"(ansi green)($icon) ($cap)%(ansi reset)"
        }
    } | default '')

    let load_segment = (do -i {
        let l = (sys cpu | get 0.load_average | split row ',' | first | into float | math round --precision 1)
        $"(ansi yellow)\u{2699} ($l)(ansi reset)"
    } | default '')

    let jobs_segment = (do -i {
        let n = (job list | length)
        if $n > 0 { $" (ansi yellow)\u{23F3} ($n)(ansi reset)" } else { '' }
    } | default '')

    let time_segment = (do -i {
        $"(ansi magenta)(date now | format date '%H:%M:%S')(ansi reset)"
    } | default '')

    [$battery_segment $load_segment $jobs_segment $time_segment]
    | where {|s| $s | is-not-empty }
    | str join " \u{2502} "
}
