#!/bin/bash

# Handle errors
err() {
    echo -e "\e[;91mEncountered an error while trying to install yanu! $(if [[ -n "$@" ]]; then echo -e "\nCause: $@"; fi)\e[0m"
    exit 1
}

# proot alias
proot() {
    echo -e "Running in PROOT:\n$*" # wtf is '$*'?
    proot-distro login ubuntu -- "$@"
}

# Setting up deps
termux-setup-storage || err "Failed to get permission to Internal storage."
pkg update || err "Failed to sync package repos."
pkg upgrade -y || err "Failed to update packages."
pkg in proot-distro || err "Failed to install proot-distro."
proot-distro install ubuntu || echo 'NOTE: `ubuntu` already installed.'
proot apt update -y || err "Failed to sync package repos in proot."
proot apt upgrade -y || err "Failed to update packages in proot."
proot apt install git gcc binutils make || err "Failed to install required deps in proot."
proot rm -f /usr/bin/yanu || err "Failed to clean up old yanu in proot."
proot curl -sLo /usr/bin/yanu https://github.com/nozwock/yanu/releases/latest/download/yanu-aarch64-termux_proot-linux-musl || err "Failed to fetch yanu binary in proot."
proot chmod +x /usr/bin/yanu || err "Failed to give executable permission."

# Setting up entry script
rm -f "$PATH/yanu" || err "Failed to clean up old entry script."
echo '
#!/bin/bash
proot-distro login --bind /storage/emulated/0 --termux-home ubuntu -- yanu "$@"' >>"$PATH/yanu" || err "Failed to write entry script."
chmod +x "$PATH/yanu" || err "Failed to give executable permission."

echo -e "\e[;92mInstalled \`yanu\` successfully\nYou can run it by typing in\n\e[0m\e[;96myanu\e[0m"
