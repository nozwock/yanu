#!/bin/bash

# Handle errors
err() {
    printf >&2 "\e[;91m%s\n\e[0m" "Error: $(if [[ -n "$*" ]]; then echo -e "$*"; else echo 'An error occurred!'; fi)"
    exit 1
}

# Alias for run in proot
# @args string
proot() {
    printf >&2 "\e[1;97m%s\n%s\n\e[0m" "Running in PROOT:" "$1"
    proot-distro login ubuntu -- bash -c "$1"
}

# Setting up deps
termux-setup-storage <<<"Y" || err "Failed to get permission to Internal storage"
pkg update || err "Failed to sync package repos"
pkg upgrade -y || err "Failed to update packages"
pkg in proot-distro || err "Failed to install proot-distro"
proot-distro install ubuntu || echo -e "\e[;34mNOTE: \`ubuntu\` seems to be already installed\e[0m"
proot 'yes Y | apt update && apt upgrade' || err "Failed to update packages in proot"
proot 'apt install git gcc binutils make -y' || err "Failed to install required deps in proot"

proot 'rm -f /usr/bin/yanu' || err "Failed to clean up old yanu in proot"
proot 'curl -sLo /usr/bin/yanu https://github.com/nozwock/yanu/releases/latest/download/yanu-aarch64-termux_proot-linux-musl' || err "Failed to fetch yanu binary in proot"
proot 'chmod +x /usr/bin/yanu' || err "Failed to give executable permission"

# Setting up entry script
rm -f "$PATH/yanu" || err "Failed to clean up old entry script"
echo '
#!/bin/bash
proot-distro login --bind /storage/emulated/0 --termux-home ubuntu -- yanu "$@"' >>"$PATH/yanu" || err "Failed to write entry script"
chmod +x "$PATH/yanu" || err "Failed to give executable permission."

echo -e "\e[;92mInstalled \`yanu\` successfully\nYou can run it by typing in\n\e[0m\e[;96myanu\e[0m"
