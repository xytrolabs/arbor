# Quality-of-life helpers for interactive use.
#
# Directory bookmarks (`mark`, `jump`, `marks`, `unmark`), fast ancestor
# navigation (`up`), directory creation plus navigation (`mkcd`), timing a
# block (`timer`), and a pointer to the Arbor documentation (`docs`).
#
# Bookmarks live in a small JSON file under `$nu.data-dir`, so they persist
# across sessions and are shared by every Arbor instance.

# Path of the bookmark file.
export def marks-file [] {
    let dir = ($nu.data-dir | path join "arbor")
    mkdir $dir
    $dir | path join "marks.json"
}

# Read bookmarks, tolerating a missing or malformed file.
def load-marks [] {
    let file = (marks-file)
    if not ($file | path exists) {
        return {}
    }
    (do -i { open $file } | default {})
}

def save-marks [marks: record] {
    let file = (marks-file)
    $marks | save -f $file
}

# Bookmark the current directory.
#
# With no name the directory's basename is used, so `mark` in
# `~/src/arbor` creates a mark called `arbor`.
@example "bookmark the current directory under its own name" { mark }
@example "bookmark the current directory as 'work'" { mark work }
export def --env mark [
    name?: string   # Name for the bookmark; defaults to the directory name.
] {
    let dir = $env.PWD
    let key = ($name | default ($dir | path basename))
    if ($key | is-empty) {
        error make { msg: "cannot bookmark the filesystem root without a name" }
    }
    mut marks = (load-marks)
    $marks = ($marks | insert $key $dir)
    save-marks $marks
    print $"marked (ansi green)($key)(ansi reset) -> ($dir)"
}

# List directory bookmarks.
export def marks [] {
    let marks = (load-marks)
    if ($marks | is-empty) {
        print "no marks yet — run `mark` in a directory to add one"
        return
    }
    $marks
    | transpose key path
    | sort-by key
}

# Change to a bookmarked directory.
@example "jump to the 'arbor' mark" { jump arbor }
export def --env jump [
    name?: string   # Bookmark to change to. Lists marks when omitted.
] {
    let marks = (load-marks)
    if ($name | is-empty) {
        if ($marks | is-empty) {
            print "no marks yet — run `mark` in a directory to add one"
            return
        }
        $marks | transpose key path | sort-by key
        return
    }
    let target = ($marks | get --optional $name)
    if $target == null {
        let available = ($marks | columns | str join ', ')
        error make {
            msg: $"no bookmark '($name)'"
            help: (if ($available | is-empty) { "add one with `mark`" } else { $"available: ($available)" })
        }
    }
    if not ($target | path exists) {
        error make { msg: $"bookmarked directory no longer exists: ($target)" }
    }
    cd $target
}

# Remove a bookmark.
export def unmark [
    name: string   # Bookmark to delete.
] {
    mut marks = (load-marks)
    if ($marks | get --optional $name) == null {
        error make { msg: $"no bookmark '($name)'" }
    }
    $marks = ($marks | reject $name)
    save-marks $marks
    print $"removed (ansi yellow)($name)(ansi reset)"
}

# Go up one or more parent directories.
@example "go up one directory" { up }
@example "go up three directories" { up 3 }
export def --env up [
    levels: int = 1   # How many parents to traverse.
] {
    if $levels < 1 {
        error make { msg: "levels must be 1 or greater" }
    }
    let target = (1..$levels | reduce --fold $env.PWD {|_it, acc| $acc | path dirname })
    cd $target
}

# Create a directory (and its parents) and change into it.
@example "create and enter a scratch dir" { mkcd /tmp/scratch/one }
export def --env mkcd [
    dir: path   # Directory to create and enter.
] {
    mkdir $dir
    let target = ($dir | path expand)
    cd $target
}

# Run a block and report how long it took.
#
# Stateless: the timing lives in the command, not in the environment, so it
# works the same in a script and at the prompt.
@example "time a sleep" { timer { sleep 100ms } }
export def timer [
    body: closure   # Block to run and time.
] {
    let started = (date now)
    let result = (do $body)
    let elapsed = ((date now) - $started)
    print $"took (ansi yellow)($elapsed)(ansi reset)"
    $result
}

# Point at the Arbor documentation.
#
# Prints the docs index, or a specific guide's location. Nothing is opened, so
# this is safe in scripts and on machines without a browser.
@example "list the documentation" { docs }
@example "locate the configuration guide" { docs 04-configuration }
export def docs [
    topic?: string   # Guide to look up, e.g. `02-usage`.
] {
    let base = "https://xytro.site/arbor/docs/"
    if ($topic | is-not-empty) {
        let url = ($base + $topic + ".md")
        print $url
        return
    }
    print $"Arbor documentation: (ansi green)($base)(ansi reset)"
    [
        "01-getting-started  install, first commands, directories"
        "02-usage            commands, pipelines, jobs, Ctrl+B, bash blocks"
        "03-scripting        Indent scripts"
        "04-configuration    every setting"
        "05-prompt           prompt customisation"
        "06-faq              troubleshooting"
    ] | each {|line| print $"  ($line)" }
}
