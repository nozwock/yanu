#!/bin/bash

# Handle errors
err() {
    echo -e "\e[;91mEncountered an error while trying to install yanu!\e[0m"
    exit 1
}

# proot alias
proot() {
    echo -e "Running in PROOT:\n$*" # wtf is '$*'?
    proot-distro login ubuntu -- "$@"
}


# Setting up deps
termux-setup-storage || err
pkg upgrade -y || err
pkg in proot-distro || err
proot-distro install ubuntu || err
proot apt update -y || err
proot apt upgrade -y || err
proot rm -f /usr/bin/yanu || err
proot curl -sLo /usr/bin/yanu https://github.com/nozwock/yanu/releases/latest/download/yanu-aarch64-termux_proot-linux-musl || err
proot chmod +x /usr/bin/yanu || err

# Setting up proxy script
rm -f "$PATH/yanu" || err
echo '
#!/bin/bash
proot-distro login --bind /storage/emulated/0 --termux-home ubuntu -- yanu "$@"' >> "$PATH/yanu" || err
chmod +x "$PATH/yanu" || err

echo -e "\e[;92mInstall \`yanu\` successfully\nYou can run it by typing in\n\e[0m\e[;96myanu\e[0m"
