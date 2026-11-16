# sysinfo — a neofetch-style system summary for Arbor.
#
# Prints a compact, colourised overview of the machine: host, OS, kernel,
# uptime, CPU, memory, disk and temperature. Every lookup is wrapped in
# `do -i` so a platform without the data (or a container) degrades to a dash
# instead of erroring.

@category "default"
@search-terms "neofetch" "system" "info" "sysinfo"
export def main [] {
    let use_colors = (config use-colors)
    let r = if $use_colors { (ansi reset) } else { "" }

    let host = (do -i { sys host } | default {})
    let mem = (do -i { sys mem } | default {})
    let cpu = (do -i { sys cpu | first } | default {})

    let hostname = (do -i { $host.hostname | str trim } | default "unknown")
    let os = (do -i { $host.long_os_version } | default (do -i { $host.name } | default "unknown"))
    let kernel = (do -i { $host.kernel_version } | default "-")
    let uptime = (do -i { $host.uptime } | default "-")
    let cpu_name = (do -i { $cpu.brand | str trim } | default "-")
    let cpu_load = (do -i { $cpu.load_average | split row ',' | first | str trim } | default "-")

    let mem_used = (do -i { $mem.used | into filesize } | default 0b)
    let mem_total = (do -i { $mem.total | into filesize } | default 0b)
    let mem_pct = (do -i {
        let u = ($mem.used | into int)
        let t = ($mem.total | into int)
        if $t > 0 { ($u / $t * 100 | math round | into int) } else { -1 }
    } | default (-1))

    let disk = (do -i {
        sys disks | where mount == "/" | first
    } | default {})
    let disk_used = (do -i { ($disk.total | into int) - ($disk.free | into int) | into filesize } | default 0b)
    let disk_total = (do -i { $disk.total | into filesize } | default 0b)

    let temp = (do -i {
        sys temp | where unit =~ "Package" | first | get temp | math round
    } | default "-")

    let cyan = if $use_colors { (ansi cyan) } else { "" }
    let green = if $use_colors { (ansi green) } else { "" }
    let yellow = if $use_colors { (ansi yellow) } else { "" }

    [
        $"($cyan)Host($r)    ($green)($hostname)($r)"
        $"($cyan)OS($r)      ($green)($os)($r)"
        $"($cyan)Kernel($r)  ($green)($kernel)($r)"
        $"($cyan)Uptime($r)  ($green)($uptime)($r)"
        $"($cyan)CPU($r)     ($green)($cpu_name)($r)  ($yellow)load ($cpu_load)($r)"
        $"($cyan)Memory($r)  ($green)($mem_used)($r) / ($green)($mem_total)($r)  ($yellow)($mem_pct)%($r)"
        $"($cyan)Disk /($r)  ($green)($disk_used)($r) / ($green)($disk_total)($r)"
        $"($cyan)Temp($r)    ($green)($temp)($r)°C"
    ] | str join (char nl)
}
