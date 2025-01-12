#!/bin/bash

# Show error message and exit
# $* - error message
err() {
    printf >&2 "\e[;91m%s\n\e[0m" "Error: $(if [[ -n "$*" ]]; then echo -e "$*"; else echo 'an error occurred'; fi)"
    exit 1
}

# Runs passed command in proot
# $1 - command string
proot() {
    printf >&2 "\e[1;97m%s\n%s\n\e[0m" "Running in PROOT:" "$1"
    proot-distro login ubuntu -- bash -c "$1"
}

# Patches to make yanu work
apply_workaround_patches() {
    # Workaround for https://github.com/nozwock/yanu/issues/44
    proot '
    if [[ ! -f "/usr/lib/aarch64-linux-gnu/libbfd-2.38-system.so" ]]; then
        ln -sf "$(find /usr/lib/aarch64-linux-gnu/ -type f -name '"'libbfd*.so'"' | head -n1)" '"'/usr/lib/aarch64-linux-gnu/libbfd-2.38-system.so'
    fi
    "
}

# Patch activity manager for performance improvements
# https://github.com/termux/termux-api/issues/552#issuecomment-1382722639
patch_am() {
    local am_path="$PREFIX/bin/am" pat="app_process" patch="-Xnoimage-dex2oat"
    sed -i "/$pat/!b; /$patch/b; s/$pat/& $patch/" "$am_path" || return $?
}

main() {
    # Argparsing
    getopt -T
    if [ "$?" != 4 ]; then
        err "wrong version of 'getopt' detected"
    fi

    set -uo noclobber -o pipefail
    # NOTE: getopt doesn't work when there's no '-o' and only `-l`
    params="$(getopt -o t: -l tag:,skip-deps --name "$0" -- "$@")" || exit $?
    eval set -- "$params"

    flag_skip_deps=false
    while (($#)); do
        case "$1" in
        -t | --tag)
            arg_tag=$2
            shift
            ;;
        --skip-deps)
            flag_skip_deps=true
            ;;
        --)
            shift
            break
            ;;
        *)
            err "Unknown: $1"
            ;;
        esac
        shift
    done
    # Argparsing - END

    USR_DIR='/data/data/com.termux/files/usr'
    BIN_DIR="${USR_DIR}/bin"

    termux-setup-storage <<<"Y" || err "Failed to get permission to Internal storage"

    # Setup deps
    if [ "$flag_skip_deps" = true ]; then
        echo -e '\e[1;93mWARN: Skipping dependencies may cause issues!\e[0m' >&2
    else
        sh -c 'yes Y | pkg update' || termux-change-repo && sh -c 'yes Y | pkg update' || err "Failed to sync package repos; Changing mirror should help 'termux-change-repo'"
        sh -c 'yes Y | pkg upgrade' || err "Failed to update packages"
        sh -c 'yes Y | pkg in proot-distro termux-api' || err "Failed to install essential packages"
        proot-distro install ubuntu || true # ignore err
        proot 'yes Y | apt update && apt upgrade' || err "Failed to update packages in proot"
        proot 'apt install git gcc binutils make -y' || err "Failed to install required deps in proot"
    fi

    proot 'which eget' || (proot '{ curl https://zyedidia.github.io/eget.sh | bash; } && mv ./eget /bin/' || err "Failed install 'eget' in proot")

    # Fetch 'yanu' binary
    proot 'rm -f /usr/bin/yanu /bin/yanu' || err "Failed to remove existing 'yanu' in proot"

    # Previous variable check method: https://stackoverflow.com/a/13864829
    if [ -v arg_tag ]; then
        # Exists
        proot "eget https://github.com/nozwock/yanu/ --asset aarch64 --tag=$arg_tag --to=/usr/bin/" || err "Failed to fetch 'yanu' binary in proot"
    else
        proot 'eget https://github.com/nozwock/yanu/ --asset aarch64 --to=/usr/bin/' || err "Failed to fetch 'yanu' binary in proot"
    fi

    apply_workaround_patches || err "Failed to apply workaround patches"
    patch_am || err "Failed to patch AM"

    # Setup entry script
    rm -f "$BIN_DIR/yanu" || err "Failed to clean up old entry script"
    rm -f "$BIN_DIR/yanu-cli" || err "Failed to clean up old entry script"

    cat >"$BIN_DIR/yanu" <<"EOF"
#!/bin/bash

CFG_DIR="$HOME/.config/com.github.nozwock.yanu"
YANU_OUT_PATH="$HOME/tmp.com.github.nozwock.yanu.out"
BINDINGS_PATH="$CFG_DIR/proot-bindings"

# Launch proot yanu
yanu() {
    bind_opts=()
    external_storage_arr=()

    if [ -d "$HOME/storage/external-1" ]; then
        external_storage_path="$(grep -E '\s/storage/.{4}-.{4}' /proc/mounts | cut -d ' ' -f2)"

        readarray -t arr <<< "$external_storage_path"
        external_storage_arr+=( "${arr[@]}" )
    fi

    if [ -f "$BINDINGS_PATH" ]; then
        readarray -t arr < "$BINDINGS_PATH"
        external_storage_arr+=( "${arr[@]}" )
    fi

    for it in "${external_storage_arr[@]}"; do
        if [ -d "$it" ]; then
            bind_opts+=( --bind "$it" )
        fi
    done

    proot-distro login ubuntu --termux-home --bind /storage/emulated/0 "${bind_opts[@]}" -- bash -c 'yanu '"$*"" 2> >(tee $YANU_OUT_PATH)"
}

filter_ansi_codes() {
    sed -r 's/\x1b\[\??[0-9;]*[A-Za-z]//g'
}

termux_api_exists=false
pkg 2>/dev/null list-installed | grep -q termux-api && ! termux-api-start 2>&1 >/dev/null | grep -iq error && termux_api_exists=true

# Shows notification for failure or success based on the status code
# $1 - status code
notify() {
    if [ "$termux_api_exists" = true ]; then
        if [ "$1" -eq 0 ]; then
            yanu_out_content="$(cat "$YANU_OUT_PATH" | tail -n2 | filter_ansi_codes)"
            message="$(echo "$yanu_out_content" | head -n1)"
            time_taken="$(echo "$yanu_out_content" | sed -nr 's/.*Process completed \((.*)\).*/\1/p')"

            echo -e "Process completed successfully\n$message\nTook $time_taken" | termux-notification -t "Yanu - Ok" --icon done
        else
            termux-notification -t "Yanu - Error" -c "Process failed due to some error" --icon error
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

notify_flag=false
for arg in "$@"; do
    if [[ $arg =~ ^(update|pack|unpack|convert|tui)$ ]]; then
        notify_flag=true
    fi
    if [[ $arg =~ ^(help|-h|--help)$ ]]; then
        notify_flag=false
        break
    fi
done

# Only set wakelock for non-help commands
if [ "$notify_flag" = true ] || [[ $# -eq 0 ]]; then
    get_wakelock
fi
trap cleanup EXIT

echo 'Entering proot...'
if [[ $# -eq 0 ]]; then
    yanu tui
    notify $?
else
    yanu "$@"
    code=$?

    if [ "$notify_flag" = true ]; then
        notify "$code"
    fi
fi
EOF

    chmod +x "$BIN_DIR/yanu" || err "Failed to give executable permission"

    echo -e "\nYanu has been successfully installed! The \e[1;92m'yanu --help'\e[0m command provides help for all available commands. (\e[1;92m'yanu-cli'\e[0m is deprecated.)" \
        "For interactive NSP updates, you can simply type \e[1;92m'yanu'\e[0m, which is an alias for \e[1;92m'yanu tui'\e[0m."
}

main "$@"
