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

BIN_PATH='/data/data/com.termux/files/usr/bin'

# Setup deps
termux-setup-storage <<<"Y" || err "Failed to get permission to Internal storage"
sh -c 'yes Y | pkg update' || termux-change-repo && sh -c 'yes Y | pkg update' || err "Failed to sync package repos; Changing mirror should help 'termux-change-repo'"
sh -c 'yes Y | pkg upgrade' || err "Failed to update packages"
pkg in proot-distro || err "Failed to install 'proot-distro'"
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

# Setup entry script
rm -f "$BIN_PATH/yanu" || err "Failed to clean up old entry script"
rm -f "$BIN_PATH/yanu-cli" || err "Failed to clean up old entry script"
echo '#!/bin/bash
proot-distro login --bind /storage/emulated/0 --termux-home ubuntu -- yanu "$@"' >>"$BIN_PATH/yanu-cli" || err "Failed to write entry script"
chmod +x "$BIN_PATH/yanu-cli" || err "Failed to give executable permission"
echo '#!/bin/bash
yanu-cli tui' >>"$BIN_PATH/yanu" || err "Failed to write alias script"
chmod +x "$BIN_PATH/yanu" || err "Failed to give executable permission"

echo -e "Yanu has been successfully installed! The \e[1;92m'yanu-cli'\e[0m command provides access to all available options." \
    "For interactive NSP updates, you can simply type \e[1;92m'yanu'\e[0m, which is an alias for \e[1;92m'yanu-cli tui'\e[0m."
