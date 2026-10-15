#!/bin/sh
# Launch alba in its own macOS Terminal window, screenshot it, and stop it.
#
# usage: terminal.sh launch HOME [--theme light|dark] [--cols N] [--rows N] [--full-screen] [--offline]
#        terminal.sh shot HOME FILE.png
#        terminal.sh stop HOME
#        terminal.sh doctor
#
# HOME is a scratch directory from the fixture example. launch writes alba.pid, window.id, and
# run.sh into it. launch refuses while Terminal runs, so every Terminal window belongs
# to this run, and stop can quit Terminal. One run at a time.
set -eu

REPO=$(cd "$(dirname "$0")/../../../.." && pwd -P)
BINARY="$REPO/target/release/alba"
DRIVER="$REPO/target/release/examples/drive"
BUILD="cargo build --release --bins --examples"
OFFLINE_PROFILE='(version 1)(allow default)(deny network*)'

die() {
	echo "terminal.sh: $*" >&2
	exit 1
}

# Prints "profile|background" for a theme. Both profiles are partly see-through; a
# background set through AppleScript (RGB from 0 to 65535, no alpha) makes them opaque,
# so the desktop stays out of a shot.
theme_settings() {
	case $1 in
	light) echo "Clear Light|{65535, 65535, 65535}" ;;
	dark) echo "Pro|{0, 0, 0}" ;;
	*) die "unknown theme: $1; use light or dark" ;;
	esac
}

# A winsize field holds 1 to 65535.
check_pane_length() {
	case $1 in
	'' | *[!0-9]*) die "not a pane length: $1" ;;
	esac
	[ "$1" -ge 1 ] && [ "$1" -le 65535 ] || die "not a pane length from 1 to 65535: $1"
}

terminal_running() {
	pgrep -xq Terminal
}

sandbox_prefix() {
	echo "/usr/bin/sandbox-exec -p '$OFFLINE_PROFILE'"
}

# run.sh records alba's pid, then becomes alba, so the pid stays the one to stop.
write_run_script() {
	sandbox=""
	[ -n "$2" ] && sandbox=$(sandbox_prefix)
	cat >"$1/run.sh" <<-EOF
		#!/bin/sh
		echo \$\$ > '$1/alba.pid'
		exec env HOME='$1' XDG_CONFIG_HOME= XDG_CACHE_HOME= XDG_STATE_HOME= $sandbox '$BINARY'
	EOF
	chmod +x "$1/run.sh"
}

# Prints the id of alba's window. The tab's terminal device finds it: the startup window
# can be in front.
open_window() {
	home=$1 settings=$2 cols=$3 rows=$4 title=$5
	osascript <<-EOF
		tell application "Terminal"
			set t to do script "exec '$home/run.sh'"
			set alba_tty to tty of t
			set alba_window to id of first window whose tty of selected tab is alba_tty
			set current settings of t to settings set "${settings%%|*}"
			set background color of t to ${settings#*|}
			set custom title of t to "$title"
			set title displays custom title of t to true
			set title displays device name of t to false
			set title displays shell path of t to false
			set title displays window size of t to false
			set number of columns of t to $cols
			set number of rows of t to $rows
			return alba_window
		end tell
	EOF
}

# Prints alba's pane as "COLSxROWS", read from its terminal.
pane_size() {
	tty=$(ps -o tty= -p "$1" | tr -d ' ')
	stty -f "/dev/$tty" size | awk '{ print $2 "x" $1 }'
}

# Zooms the window to fill the screen, less the menu bar and the Dock.
fill_screen() {
	osascript -e "tell application \"Terminal\" to set zoomed of window id $1 to true" >/dev/null
}

alive() {
	[ -f "$1" ] && kill -0 "$(cat "$1")" 2>/dev/null
}

launch() {
	home=$1
	shift
	theme=dark cols=85 rows=26 offline="" full_screen=""
	while [ $# -gt 0 ]; do
		case $1 in
		--theme) theme=$2 && shift 2 ;;
		--cols) cols=$2 && shift 2 ;;
		--rows) rows=$2 && shift 2 ;;
		--full-screen) full_screen=1 && shift ;;
		--offline) offline=1 && shift ;;
		*) die "unknown option: $1" ;;
		esac
	done
	settings=$(theme_settings "$theme")
	check_pane_length "$cols"
	check_pane_length "$rows"
	[ -f "$home/.config/alba/config.toml" ] || die "$home has no config; run the fixture example first"
	[ -x "$BINARY" ] || die "no release build; run $BUILD"
	alive "$home/alba.pid" && die "alba already runs from $home; stop it first"
	terminal_running && die "Terminal is running; stop the other run, or ask the user to quit Terminal"

	write_run_script "$home" "$offline"
	rm -f "$home/alba.pid" "$home/window.id"
	size="${cols}x${rows}"
	[ -n "$full_screen" ] && size="full screen"
	title="alba · $theme · $size${offline:+ · offline}"
	# Terminal also opens a startup window; it stays until stop, and shot leaves it out.
	open_window "$home" "$settings" "$cols" "$rows" "$title" >"$home/window.id"
	[ -n "$full_screen" ] && fill_screen "$(cat "$home/window.id")"

	for _ in $(seq 50); do
		alive "$home/alba.pid" && break
		sleep 0.2
	done
	alive "$home/alba.pid" || die "alba did not start; read $home/run.sh"
	# The resize redraw follows at once; a second lets the fetch indicator settle.
	sleep 1
	pane=$(pane_size "$(cat "$home/alba.pid")")
	[ -n "$full_screen" ] || [ "$pane" = "${cols}x${rows}" ] ||
		echo "warning: the pane is $pane, not ${cols}x${rows}; a window manager resized the window" >&2
	echo "alba $(cat "$home/alba.pid") in Terminal window $(cat "$home/window.id"): $title, pane $pane"
}

shot() {
	home=$1 file=$2
	[ -s "$home/window.id" ] || die "no Terminal window from $home; launch first"
	mkdir -p "$(dirname "$file")"
	# -l captures only this window, so no other window, shadow, or border gets in.
	screencapture -x -o -l "$(cat "$home/window.id")" "$file"
	echo "$file"
}

# Kills what launch started: alba, then Terminal, which launch started too.
# Terminal's AppleScript can close a window other than the one named, so stop quits it.
stop() {
	home=$1
	if alive "$home/alba.pid"; then
		kill "$(cat "$home/alba.pid")"
		sleep 0.5
	fi
	if [ -s "$home/window.id" ] && terminal_running; then
		osascript -e 'tell application "Terminal" to quit'
	fi
	rm -f "$home/alba.pid" "$home/window.id"
	echo "stopped $home"
}

doctor() {
	echo "macOS $(sw_vers -productVersion)"
	[ -x "$BINARY" ] || die "no release build; run $BUILD"
	"$BINARY" --version
	newer=$(find "$REPO/src" "$REPO/Cargo.toml" -newer "$BINARY" | head -1)
	[ -z "$newer" ] || echo "stale build: $newer is newer than $BINARY; run $BUILD"
	[ -x "$DRIVER" ] || die "no headless driver; run $BUILD"
	newer=$(find "$REPO/src" "$REPO/tests/common" "$REPO/examples" "$REPO/Cargo.toml" -newer "$DRIVER" | head -1)
	[ -z "$newer" ] || echo "stale build: $newer is newer than $DRIVER; run $BUILD"
	found=""
	for pidfile in /tmp/alba-verify.*/alba.pid; do
		[ -f "$pidfile" ] || continue
		found=1
		state=stopped
		alive "$pidfile" && state="running, pid $(cat "$pidfile")"
		echo "run $(dirname "$pidfile"): $state"
	done
	[ -n "$found" ] || echo "no verification runs"
}

command=${1:-}
[ $# -gt 0 ] && shift
case $command in
launch) [ $# -ge 1 ] || die "usage: launch HOME [options]"; launch "$@" ;;
shot) [ $# -eq 2 ] || die "usage: shot HOME FILE.png"; shot "$@" ;;
stop) [ $# -eq 1 ] || die "usage: stop HOME"; stop "$@" ;;
doctor) doctor ;;
*) die "usage: terminal.sh launch|shot|stop|doctor" ;;
esac
