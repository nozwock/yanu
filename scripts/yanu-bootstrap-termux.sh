#!/bin/bash

# Handle errors
err() {
    printf >&2 "\e[;91m%s\n\e[0m" "Error: $(if [[ -n "$*" ]]; then echo -e "$*"; else echo 'an error occurred'; fi)"
    exit 1
}

# Alias for run in proot
# @args string
proot() {
    printf >&2 "\e[1;97m%s\n%s\n\e[0m" "Running in PROOT:" "$1"
    proot-distro login ubuntu -- bash -c "$1"
}

patch_am() {
    # https://github.com/termux/termux-api/issues/552#issuecomment-1382722639
    local am_path="$PREFIX/bin/am" pat="app_process" patch="-Xnoimage-dex2oat"
    sed -i "/$pat/!b; /$patch/b; s/$pat/& $patch/" "$am_path" || return $?
}

# Argparsing
getopt -T
if [ "$?" != 4 ]; then
    err "wrong version of 'getopt' detected"
fi

set -uo noclobber -o pipefail
params="$(getopt -o t: -l tag: --name "$0" -- "$@")" || exit $?
eval set -- "$params"

while true; do
    case "$1" in
    -t | --tag)
        arg_tag=$2
        shift 2
        ;;
    --)
        shift
        break
        ;;
    *)
        err "Unknown: $1"
        ;;
    esac
done
# Argparsing - END

USR_DIR='/data/data/com.termux/files/usr'
BIN_DIR="${USR_DIR}/bin"

# Setup deps
termux-setup-storage <<<"Y" || err "Failed to get permission to Internal storage"
sh -c 'yes Y | pkg update' || termux-change-repo && sh -c 'yes Y | pkg update' || err "Failed to sync package repos; Changing mirror should help 'termux-change-repo'"
sh -c 'yes Y | pkg upgrade' || err "Failed to update packages"
sh -c 'yes Y | pkg in proot-distro termux-api' || err "Failed to install essential packages"
proot-distro install ubuntu || true # ignore err
proot 'yes Y | apt update && apt upgrade' || err "Failed to update packages in proot"
proot 'apt install git gcc binutils make -y' || err "Failed to install required deps in proot"
proot 'which eget' || (proot '{ curl https://zyedidia.github.io/eget.sh | bash; } && mv ./eget /bin/' || err "Failed install 'eget' in proot")

# Fetch 'yanu' binary
proot 'rm -f /usr/bin/yanu /bin/yanu' || err "Failed to remove existing 'yanu' in proot"
if [ -z ${arg_tag+x} ]; then # https://stackoverflow.com/a/13864829
    # Unset
    proot 'eget https://github.com/nozwock/yanu/ --asset aarch64 --to=/usr/bin/' || err "Failed to fetch 'yanu' binary in proot"
else
    # Set
    proot "eget https://github.com/nozwock/yanu/ --asset aarch64 --tag=$arg_tag --to=/usr/bin/" || err "Failed to fetch 'yanu' binary in proot"
fi

# Patch activity manager for performance improvements
patch_am

# Setup entry script
rm -f "$BIN_DIR/yanu" || err "Failed to clean up old entry script"
rm -f "$BIN_DIR/yanu-cli" || err "Failed to clean up old entry script"

echo $'#!/bin/bash

YANU_OUT_PATH="$HOME/tmp.com.github.nozwock.yanu.out"

yanu() {
    proot-distro login ubuntu --bind /storage/emulated/0 --termux-home -- bash -c "$(echo "yanu ""$@"" 2> >(tee $YANU_OUT_PATH)")"
}

filter_ansi_codes() {
    sed -r \'s/\\x1b\[\\??[0-9;]*[A-Za-z]//g\'
}

termux_api_exists=false
pkg 2>/dev/null list-installed | grep -q termux-api && ! termux-api-start 2>&1 >/dev/null | grep -iq error && termux_api_exists=true

# $1 - status code
notify() {
    if $termux_api_exists; then
        if [ "$1" -eq 0 ]; then
            yanu_out_content="$(cat "$YANU_OUT_PATH" | tail -n2 | filter_ansi_codes)"
            message="$(echo "$yanu_out_content" | head -n1)"
            time_taken="$(echo "$yanu_out_content" | sed -nr \'s/.*Process completed \((.*)\).*/\\1/p\')"

            echo -e "Process completed successfully\\n$message\\nTook $time_taken" | termux-notification -t "Yanu" --icon done
        else
            termux-notification -t "Yanu" -c "Process failed due to some error" --icon error
        fi
    fi
}

get_wakelock() {
    echo "Acquiring wakelock..."
    termux-wake-lock
}

cleanup() {
    rm -f "$YANU_OUT_PATH"
    termux-wake-unlock
    exit
}

get_wakelock
trap cleanup EXIT

if [[ $# -eq 0 ]]; then
    yanu tui
    notify $?
else
    yanu "$@"
    code=$?

    notify_flag=false
    for arg in "$@"; do
        if [[ $arg =~ ^(update|pack|unpack|convert|tui)$ ]]; then
            notify_flag=true
        fi
        if [[ $arg =~ ^(-h|--help)$ ]]; then
            notify_flag=false
            break
        fi
    done

    if $notify_flag; then
        notify $code
    fi
fi
' >>"$BIN_DIR/yanu" || err "Failed to write entry script"
chmod +x "$BIN_DIR/yanu" || err "Failed to give executable permission"

echo -e "Yanu has been successfully installed! The \e[1;92m'yanu-cli'\e[0m command provides access to all available options." \
    "For interactive NSP updates, you can simply type \e[1;92m'yanu'\e[0m, which is an alias for \e[1;92m'yanu-cli tui'\e[0m."
