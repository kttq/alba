#!/bin/sh
# Launch alba in its own Ghostty window, screenshot it, and stop it.
#
# usage: ghostty.sh launch HOME [--theme NAME] [--cols N] [--rows N] [--offline]
#        ghostty.sh shot HOME FILE.png
#        ghostty.sh stop HOME
#        ghostty.sh doctor
#
# HOME is a scratch directory from fixture.py. launch writes alba.pid, ghostty.pid, and
# run.sh into it. Each launch is a separate Ghostty process with no user config, so the
# shot depends only on the theme and the size.
set -eu

REPO=$(cd "$(dirname "$0")/../../../.." && pwd -P)
BINARY="$REPO/target/release/alba"
GHOSTTY=/Applications/Ghostty.app
OFFLINE_PROFILE='(version 1)(allow default)(deny network*)'
# Top-left corner of the window, in screen points.
POSITION_X=80
POSITION_Y=80

die() {
	echo "ghostty.sh: $*" >&2
	exit 1
}

# Prints "number x,y,width,height" of the on-screen window that a process owns.
window() {
	osascript -l JavaScript -e "
		ObjC.import('CoreGraphics');
		const windows = ObjC.deepUnwrap(ObjC.castRefToObject(
			\$.CGWindowListCopyWindowInfo(\$.kCGWindowListOptionOnScreenOnly, 0)));
		const own = windows.filter(w => w.kCGWindowOwnerPID === $1 && w.kCGWindowLayer === 0);
		const bounds = own.length && own[0].kCGWindowBounds;
		own.length ? own[0].kCGWindowNumber + ' ' +
			[bounds.X, bounds.Y, bounds.Width, bounds.Height].join(',') : '';"
}

# Walks up from alba to the Ghostty process that runs it.
ghostty_ancestor() {
	pid=$1
	while [ "$pid" -gt 1 ]; do
		case $(ps -o comm= -p "$pid") in
		*/Ghostty.app/Contents/MacOS/ghostty) echo "$pid" && return ;;
		esac
		pid=$(ps -o ppid= -p "$pid" | tr -d ' ')
	done
	return 1
}

# A tiling window manager overrides Ghostty's size; floating restores it.
float() {
	if command -v aerospace >/dev/null && aerospace list-workspaces --focused >/dev/null 2>&1; then
		aerospace layout floating --window-id "$1"
		sleep 0.3
	fi
}

# Prints the macOS sandbox-exec prefix that denies every network call.
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

alive() {
	[ -f "$1" ] && kill -0 "$(cat "$1")" 2>/dev/null
}

launch() {
	home=$1
	shift
	theme="Builtin Dark" cols=85 rows=26 offline=""
	while [ $# -gt 0 ]; do
		case $1 in
		--theme) theme=$2 && shift 2 ;;
		--cols) cols=$2 && shift 2 ;;
		--rows) rows=$2 && shift 2 ;;
		--offline) offline=1 && shift ;;
		*) die "unknown option: $1" ;;
		esac
	done
	[ -f "$home/.config/alba/config.toml" ] || die "$home has no config; run fixture.py first"
	[ -x "$BINARY" ] || die "no release build; run cargo build --release"
	alive "$home/alba.pid" && die "alba already runs from $home; stop it first"

	write_run_script "$home" "$offline"
	rm -f "$home/alba.pid" "$home/ghostty.pid"

	open -na "$GHOSTTY" --args \
		--config-default-files=false \
		"--theme=$theme" "--title=alba · $theme · ${cols}x${rows}${offline:+ · offline}" \
		"--window-width=$cols" "--window-height=$rows" \
		"--window-position-x=$POSITION_X" "--window-position-y=$POSITION_Y" \
		--window-save-state=never \
		--quit-after-last-window-closed=true \
		--confirm-close-surface=false \
		-e sh -c "exec '$home/run.sh'"

	for _ in $(seq 50); do
		alive "$home/alba.pid" && break
		sleep 0.2
	done
	alive "$home/alba.pid" || die "alba did not start; read $home/run.sh"
	ghostty_ancestor "$(cat "$home/alba.pid")" >"$home/ghostty.pid" || die "no Ghostty above alba"
	for _ in $(seq 50); do
		[ -n "$(window "$(cat "$home/ghostty.pid")")" ] && break
		sleep 0.2
	done
	float "$(window "$(cat "$home/ghostty.pid")" | cut -d' ' -f1)"
	# The first draw follows the window; a second lets the fetch indicator settle.
	sleep 1
	echo "alba $(cat "$home/alba.pid") in Ghostty $(cat "$home/ghostty.pid"): $theme, ${cols}x${rows}${offline:+, offline}"
}

shot() {
	home=$1 file=$2
	alive "$home/ghostty.pid" || die "no Ghostty from $home; launch first"
	bounds=$(window "$(cat "$home/ghostty.pid")" | cut -d' ' -f2)
	[ -n "$bounds" ] || die "the Ghostty window from $home is not on screen"
	mkdir -p "$(dirname "$file")"
	screencapture -x -R "$bounds" "$file"
	echo "$file ($bounds)"
}

# Kills only what launch started: alba, then its Ghostty if it lingers.
stop() {
	home=$1
	if alive "$home/alba.pid"; then
		kill "$(cat "$home/alba.pid")"
	fi
	for _ in $(seq 25); do
		alive "$home/ghostty.pid" || break
		sleep 0.2
	done
	if alive "$home/ghostty.pid"; then
		kill "$(cat "$home/ghostty.pid")"
	fi
	rm -f "$home/alba.pid" "$home/ghostty.pid"
	echo "stopped $home"
}

doctor() {
	[ -d "$GHOSTTY" ] || die "Ghostty is not in /Applications"
	"$GHOSTTY/Contents/MacOS/ghostty" --version | head -1
	[ -x "$BINARY" ] || die "no release build; run cargo build --release"
	"$BINARY" --version
	newer=$(find "$REPO/src" "$REPO/Cargo.toml" -newer "$BINARY" | head -1)
	[ -z "$newer" ] || echo "stale build: $newer is newer than $BINARY; run cargo build --release"
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
*) die "usage: ghostty.sh launch|shot|stop|doctor" ;;
esac
