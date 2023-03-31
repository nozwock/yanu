#!/bin/bash
global_args=($@)
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


# Interactive prompt function
# Get argument index
# USAGE: <MATCHING PATTERN> $@ <--- rest of arguments
# Thanks to bash sensei for complexity ; i giv up
find_arg_index() {
    if [ "$#" -le "1" ]; then # '$#' is total number of arguments given
        return
    fi
    for i in $(seq 1 $#); do
        if [[ "$1" == "${@:i:1}" ]]; then
            echo "$i"
            return
        fi
    done
}
###### Argparsing #######
# Error checking
# Check if the --tag arg was used twice
if [[ "$(echo "$@" | grep -o -e "--tag" | wc -l)" -gt "1" ]]; then
    echo -e "ERROR: You cannot use the same argument more than once. eg. --tag --tag"
    exit 1
fi
# Check for arguments
for i in $@; do 
    case $i in 
    "--")
    continue
    ;;
    "--tag")
        custom_release_tag="true"
        # Find out where is the --tag argument located positionally
        PROMPT_CHOICE=$(find_arg_index "--tag" $global_args)
        PROMPT_CHOICE=$(($PROMPT_CHOICE+1))
        # Now check if the next arg after --tag is empty or not
        if [[ -z "${global_args[PROMPT_CHOICE]}" ]]; then
            # If its empty throw an error straight at users face :D
            echo -e "ERROR: Invalid argument configuration! Any flags after '--tag' seems to be empty."
            exit 1
        else
            # If not then check if that version really exists in the release tags database
            # For that we *need* to fetch it first
            yanu_versioning_alt=$(git ls-remote --tags https://github.com/nozwock/yanu | cut -d "/" -f 3 | sort -r) || err "Failed to fetch available yanu release lists."
            # Now lets match against it in an efficient way (i mean no match)
            if [[ -z "$(echo "$yanu_versioning_alt" | grep -E "^${global_args[PROMPT_CHOICE]}$")" ]]; then
                # If the specified version does not exist then throw an error once again :DDD and exit
                echo -e "ERROR: The specified version in --tag flag doesn't seem to exist in the releases database; have you tried checking it?"
                exit 1
            else
                # If it matches then set tag match var to true
                # We are only Doing that cause it'll take tag arg as invalid and we dont want that
                tag_match="true"
            fi
        fi
        ;;
    *)
        if [ "$tag_match" != "true" ]; then
            echo -e "ERROR: Invalid argument $i \b!"
            exit 1
        else
            tag_match="false"
        fi
        ;;
    esac
done
###### Argparsing - END #######
############
### Set up deps
termux-setup-storage <<<"Y" || err "Failed to get permission to Internal storage."
pkg update || err "Failed to sync package repos."
pkg upgrade -y || err "Failed to update packages."
pkg in proot-distro || err "Failed to install proot-distro."
proot-distro install ubuntu || echo -e "\e[;34mNOTE: \`ubuntu\` seems to be already installed.\e[0m"
proot 'yes Y | apt update && apt upgrade' || err "Failed to update packages in proot."
proot 'apt install git gcc binutils make -y' || err "Failed to install required deps in proot."

# Check if whether eget is installed or not
if [[ -z "$(proot 'which eget')" ]]; then
    proot '{ curl https://zyedidia.github.io/eget.sh | bash; } && mv ./eget /bin/' || err "Failed to fetch and install eget binary in proot."
# If eget is already installed then remove eget and reinstall (newest version perhaps?)
else
    proot 'rm -f $(which eget)'
    proot '{ curl https://zyedidia.github.io/eget.sh | bash; } && mv ./eget /bin/' || err "Failed to fetch and install eget binary in proot."
fi
### Finish setting up deps
if [ "$custom_release_tag" == "true" ]; then
	proot 'rm -f /usr/bin/yanu /bin/yanu' || err "Failed to remove existing yanu in proot."
    proot "eget https://github.com/nozwock/yanu/ --asset aarch64 --tag="${global_args[PROMPT_CHOICE]}" --to="/usr/bin/"" || err "Failed to fetch yanu binary in proot."
else
	proot 'rm -f /usr/bin/yanu /bin/yanu' || err "Failed to remove existing yanu in proot."
    proot 'eget https://github.com/nozwock/yanu/ --asset aarch64 --to="/usr/bin/"' || err "Failed to fetch yanu binary in proot."
fi
# Setting up entry script
rm -f "$PATH/yanu" || err "Failed to clean up old entry script"
echo '
#!/bin/bash
proot-distro login --bind /storage/emulated/0 --termux-home ubuntu -- yanu "$@"' >>"$PATH/yanu" || err "Failed to write entry script"
chmod +x "$PATH/yanu" || err "Failed to give executable permission."

echo -e "\e[;92mInstalled \`yanu\` successfully\nYou can run it by typing in\n\e[0m\e[;96myanu\e[0m"
