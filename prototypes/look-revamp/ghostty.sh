#!/bin/sh
# PROTOTYPE (issue #55), throwaway: run alba in its own Ghostty instance, screenshot the window,
# and stop that instance. Other Ghostty windows are left alone.
#
# usage: ghostty.sh shot HOME FILE.png [--cols 85] [--rows 26] [--theme dark|light] [--wait 2] [VAR=VALUE ...]
# VAR=VALUE pairs go to alba's environment, such as ALBA_LOOK_THEME=warm ALBA_LOOK_TIME=07:10.
set -eu

REPO=$(cd "$(dirname "$0")/../.." && pwd -P)
BINARY="$REPO/target/release/alba"
OFFLINE_PROFILE='(version 1)(allow default)(deny network*)'

die() {
	echo "ghostty.sh: $*" >&2
	exit 1
}

window_of() {
	osascript -l JavaScript -e "
ObjC.import('CoreGraphics');
const list = ObjC.castRefToObject(\$.CGWindowListCopyWindowInfo(\$.kCGWindowListOptionOnScreenOnly, 0));
let id = '';
for (let i = 0; i < list.count; i++) {
  const w = list.objectAtIndex(i);
  if (ObjC.unwrap(w.objectForKey('kCGWindowOwnerPID')) == $1 && ObjC.unwrap(w.objectForKey('kCGWindowLayer')) == 0) id = ObjC.unwrap(w.objectForKey('kCGWindowNumber'));
}
id;"
}

shot() {
	home=$1 file=$2
	shift 2
	cols=85 rows=26 theme=dark wait=2 envs=""
	while [ $# -gt 0 ]; do
		case $1 in
		--cols) cols=$2 && shift 2 ;;
		--rows) rows=$2 && shift 2 ;;
		--theme) theme=$2 && shift 2 ;;
		--wait) wait=$2 && shift 2 ;;
		*=*) envs="$envs '$1'" && shift ;;
		*) die "unknown option: $1" ;;
		esac
	done
	[ -x "$BINARY" ] || die "no release build"
	case $theme in
	dark) colors="--background=#16130f --foreground=#e6dccb" ;;
	light) colors="--background=#fbf8f1 --foreground=#2b2620" ;;
	*) die "theme is dark or light" ;;
	esac
	cat >"$home/run.sh" <<-EOF
		#!/bin/sh
		echo \$\$ > '$home/alba.pid'
		exec env HOME='$home' XDG_CONFIG_HOME= XDG_CACHE_HOME= XDG_STATE_HOME= ALBA_LOOK_BAR=0 $envs /usr/bin/sandbox-exec -p '$OFFLINE_PROFILE' '$BINARY'
	EOF
	chmod +x "$home/run.sh"
	rm -f "$home/alba.pid"
	before=$(pgrep -x ghostty | sort)
	# shellcheck disable=SC2086
	open -na /Applications/Ghostty.app --args --config-default-files=false \
		--window-width="$cols" --window-height="$rows" --font-size=13 \
		--window-save-state=never --confirm-close-surface=false --quit-after-last-window-closed=true \
		--macos-titlebar-style=native --title="alba · $theme · ${cols}x${rows}" $colors --command="$home/run.sh"
	pid=""
	for _ in $(seq 50); do
		pid=$(pgrep -x ghostty | sort | grep -vxF "$before" | head -1 || true)
		[ -n "$pid" ] && [ -f "$home/alba.pid" ] && break
		sleep 0.2
	done
	[ -n "$pid" ] || die "Ghostty did not start"
	# AeroSpace tiles new Ghostty windows; floating this one gives it back its own size.
	if command -v aerospace >/dev/null; then
		sleep 0.5
		id=$(aerospace list-windows --all --format '%{window-id}|%{app-pid}' | awk -F'|' -v pid="$pid" '$2 == pid { print $1 }' | head -1)
		[ -n "$id" ] && aerospace layout floating --window-id "$id"
	fi
	sleep "$wait"
	window=$(window_of "$pid")
	[ -n "$window" ] || die "no window for Ghostty $pid"
	mkdir -p "$(dirname "$file")"
	screencapture -x -o -l "$window" "$file"
	tty=$(ps -o tty= -p "$(cat "$home/alba.pid")" | tr -d ' ')
	pane=$(stty -f "/dev/$tty" size 2>/dev/null | awk '{ print $2 "x" $1 }' || echo "?")
	cpu=$(ps -o cputime= -p "$(cat "$home/alba.pid")" | tr -d " ")
	kill "$(cat "$home/alba.pid")" 2>/dev/null || true
	sleep 0.3
	kill "$pid" 2>/dev/null || true
	echo "$file (pane $pane, cpu $cpu after ${wait}s)"
}

command=${1:-}
[ $# -gt 0 ] && shift
case $command in
shot) [ $# -ge 2 ] || die "usage: shot HOME FILE.png [options]"; shot "$@" ;;
*) die "usage: ghostty.sh shot HOME FILE.png [options]" ;;
esac
