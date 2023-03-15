#!/bin/bash

# Handle errors
err() {
    echo -e "\e[;91mEncountered an error while trying to install yanu! $(if [[ -n "$@" ]]; then echo -e "\nReason: $@"; fi)\e[0m"
    exit 1
}

# proot alias
proot() {
    echo -e "Running in PROOT:\n$*" # wtf is '$*'?
    proot-distro login ubuntu -- "$@"
}


# Setting up deps
termux-setup-storage || err "Failed to set up termux storage"
pkg update || err "Failed to update package lists"
pkg upgrade -y || err "Failed to upgrade packages"
pkg in proot-distro || err "Failed to install proot-distro"
proot-distro install ubuntu || echo 'NOTE: `ubuntu` already installed.'
proot apt update -y || err "Failed to update package lists in proot-distro"
proot apt upgrade -y || err "Failed to upgrade packages in proot-distro"
proot apt install git gcc binutils make || err "Failed to install required depends in proot-distro"
proot rm -f /usr/bin/yanu || err "Failed to remove yanu binary in proot-distro"
proot curl -sLo /usr/bin/yanu https://github.com/nozwock/yanu/releases/latest/download/yanu-aarch64-termux_proot-linux-musl || err "Failed to download yanu using curl in proot-distro"
proot chmod +x /usr/bin/yanu || err "Error while giving yanu bin executable permission"

# Setting up proxy script
rm -f "$PATH/yanu" || err "Failed to remove yanu binary (Proxy script)"
echo '
#!/bin/bash
proot-distro login --bind /storage/emulated/0 --termux-home ubuntu -- yanu "$@"' >> "$PATH/yanu" || err "Failed to make proxy script for yanu"
chmod +x "$PATH/yanu" || err "Failed to give executable permission to the yanu proxy script"

echo -e "\e[;92mInstalled \`yanu\` successfully\nYou can run it by typing in\n\e[0m\e[;96myanu\e[0m"
