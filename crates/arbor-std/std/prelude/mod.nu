use std/dt [datetime-diff, pretty-print-duration]

# Print a banner for Nushell with information about the project
@category "default"
@search-terms "welcome" "startup"
export def banner [
    --short            # Only show the startup time
    --no-startup-time  # Show the welcome message without the startup time
] {
let foreground = $env.config.color_config?.banner_foreground? | default "attr_normal"
let highlight1 = $env.config.color_config?.banner_highlight1? | default "green"
let startup_time = $"(ansi $highlight1)(ansi attr_bold)Startup Time: (ansi reset)(ansi $foreground)($nu.startup-time)(ansi reset)"

# The REPL prints the welcome message before the startup hooks run and the startup time right
# before the first prompt, once it is final. `--short` skips the welcome message, whose `version`
# and `datetime-diff` calls cost far more than the startup time line.
let banner_msg = if $short {
    $"($startup_time)(char eol)"
} else {
    let highlight2 = $env.config.color_config?.banner_highlight2? | default "purple"
    let dt = (datetime-diff (date now) 2019-05-10T09:59:12-07:00)
    let dt_fork = (datetime-diff (date now) 2026-11-10T00:00:00-08:00)
    let ver = (version)

    # ── System info line (Waash-style "liveliness") ──────────────────────
    # Every lookup is wrapped in `do -i` so an unsupported platform, a
    # container without `sys` data, or a missing command degrades to a dash
    # instead of breaking the banner.
    let sys_host = (do -i { sys host } | default {})
    let sys_mem = (do -i { sys mem } | default {})

    let host_name = (do -i { $sys_host.hostname | str trim } | default "unknown")
    let os_name = (do -i { $sys_host.long_os_version } | default (do -i { $sys_host.name } | default "unknown"))
    let kernel = (do -i { $sys_host.kernel_version } | default "-")
    let uptime = (do -i { $sys_host.uptime } | default "-")
    let mem_pct = (do -i {
        let used = ($sys_mem.used | into int)
        let total = ($sys_mem.total | into int)
        if $total > 0 { ($used / $total * 100 | math round | into int) } else { -1 }
    } | default (-1))
    let mem_seg = (if $mem_pct >= 0 { $"mem ($mem_pct)%" } else { "mem -" })

    let welcome = $"(ansi $highlight1)            .            (ansi reset)
(ansi $highlight1)           / \\           (ansi reset)(ansi $foreground)Welcome to arbor,(ansi reset)
(ansi $highlight1)          /   \\          (ansi reset)(ansi $foreground)structured data, structured thinking(ansi reset)
(ansi $highlight1)         /     \\         (ansi reset)
(ansi $highlight1)        /       \\        (ansi reset)(ansi $foreground)structured pipelines, built to last(ansi reset)
(ansi $highlight1)       /         \\       (ansi reset)
(ansi $highlight1)      /           \\      (ansi reset)(ansi $foreground)Ctrl+B   background any job, keep your prompt(ansi reset)
(ansi $highlight1)     /             \\     (ansi reset)(ansi $foreground)bash     multi-line blocks just work(ansi reset)
(ansi $highlight1)    /_______________\\    (ansi reset)
(ansi $highlight1)          | |            (ansi reset)(ansi $foreground).ind     Indent scripts, shell-first syntax(ansi reset)
(ansi $highlight1)          | |            (ansi reset)
(ansi $highlight1)          | |            (ansi reset)
(ansi $highlight1)    ______|_|______      (ansi reset)

(ansi $foreground)(do -i { $"($host_name) (ansi $highlight2)\u{00b7}(ansi reset)(ansi $foreground) ($os_name) (ansi $highlight2)\u{00b7}(ansi reset)(ansi $foreground) kernel ($kernel) (ansi $highlight2)\u{00b7}(ansi reset)(ansi $foreground) up ($uptime) (ansi $highlight2)\u{00b7}(ansi reset)(ansi $foreground) ($mem_seg)" } | default '')(ansi reset)

(ansi $foreground)Version: (ansi $highlight1)($ver.version) \(($ver.build_target)\)(ansi reset)
(ansi $foreground)Docs: (ansi $highlight2)https://xytro.site/arbor/(ansi reset)(ansi $foreground) | Built on Nushell: (ansi $highlight2)https://nushell.sh(ansi reset)
(ansi $foreground)Hide this with: (ansi $highlight2)`$env.config.show_banner = false`(ansi reset)

(ansi $foreground)Since Nushell's first commit: (ansi $highlight1)(pretty-print-duration $dt)(ansi reset)
(ansi $foreground)Since Arbor's first commit:  (ansi $highlight1)(pretty-print-duration $dt_fork)(ansi reset)
"

    if $no_startup_time {
        $welcome
    } else {
        $"($welcome)(char eol)($startup_time)(ansi reset)(char eol)"
    }
}

match (config use-colors) {
    false => { $banner_msg | ansi strip }
    _ => $banner_msg
}
}

# Return the current working directory
@category "default"
export def pwd [
    --physical (-P) # Resolve symbolic links
] {
    if $physical {
        $env.PWD | path expand
    } else {
        $env.PWD
    }
}
