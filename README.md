<div align="center">

# yanu
Yet Another NSP Updater for [Skyline](https://github.com/skyline-emu/skyline)

**Consider starring 🌟 the project if it helped**

</div align="center">

---

## About
[![download badge](https://img.shields.io/github/downloads/nozwock/yanu/total?style=social)](https://github.com/nozwock/yanu/releases)

**Yanu** is a **Nintendo Switch** ROM updater designed to work with Skyline on [Android w/Termux](https://github.com/nozwock/yanu#android-wtermux), [Linux](https://github.com/nozwock/yanu#linux), and [Windows](https://github.com/nozwock/yanu#windows), as well as other platforms with the help of [Docker images](https://github.com/nozwock/yanu#docker). It serves as a temporary solution until Skyline supports Updates/DLCs natively.

> **Warning** - **Doesn't support DLCs.**

Precompiled binaries are available from the [GitHub releases page](https://github.com/nozwock/yanu/releases).

<details><summary>Screenshot</summary>
<img alt="screenshot" src="https://github.com/nozwock/yanu/assets/57829219/4543d6c0-ab91-41e0-abdd-6c3ad5ec2591">
</details>

### Supported File Types
- [x] NSP 
- [x] XCI* (through XCI-NSP conversion)

## Installation

### Android w/Termux

> **Check out [Video Tutorial](https://teddit.net/r/EmulationOnAndroid/comments/11ui6v8) by SmokeyMC for Android.**

1. Download & Install `Termux` from [F-droid](https://f-droid.org/en/packages/com.termux/).
2. Copy-paste the following in `Termux` and hit enter:
  ```console
  bash <(curl -L https://raw.githubusercontent.com/nozwock/yanu/main/scripts/yanu-bootstrap-termux.sh)
  ```

Relevant resources:
- MiXplorer [XDA Forum](https://forum.xda-developers.com/t/app-2-2-mixplorer-v6-x-released-fully-featured-file-manager.1523691/)
- MiXplorer GDrive [download link](https://drive.google.com/drive/folders/1BfeK39boriHy-9q76eXLLqbCwfV17-Gv)


### Linux

1. Ensure that your system has all the necessary dependencies installed to build hactool/hacPack/etc. For example:
   ```console
   sudo apt -y install gcc-12 g++-12 make git libjpeg-dev binutils-dev libicu-dev
   ```
2. Download & give executable permission to `yanu`:
   ```console
   chmod +x yanu-x86_64-unknown-linux-musl
   ```

> [!NOTE]
> If for some reason, the backends are not compiling properly for you on Linux, you could always use the docker builds.

### Windows

- Just [download](https://github.com/nozwock/yanu/releases) & run.

> **Note**\
> Due to the lack of code-signing, some anti-virus programs may falsely identify the program as malicious. I cannot afford expensive certificates to prevent this. Exercise caution if concerned and consider using tools like [virustotal.com](https://www.virustotal.com) or [hybrid-analysis.com](https://www.hybrid-analysis.com).

### Docker
Go [here](https://github.com/nozwock/yanu/pkgs/container/yanu) to pull the container you wish to use.
Once you have the container, you can use it like this:
   ```sh
      # Expecting 'prod.keys` in pwd
      docker run -v $(pwd)/prod.keys:/root/.switch/prod.keys -v $(pwd):/work ghcr.io/nozwock/yanu update --base '/path/to/base' --update '/path/to/update' 
   ```

## Usage (CLI only)
View CLI help with:
```sh
yanu-cli --help
```

For updating a ROM:
```sh
yanu-cli --keyfile '/path/to/keyfile' update --base '/path/to/base' --update '/path/to/update'
```

Set a new `Yanu Directory` (Used in `tui`) with:
```sh
yanu-cli config --yanu-dir '/new/path/here'
```

For unpacking ROMs:
```sh
yanu-cli unpack --base '/path/to/base' --update '/path/to/update'
```

OR, for only unpacking a single ROM:
```sh
yanu-cli unpack --base '/path/to/base'
```

For packing unpacked ROM data (both base+update were unpacked):
```sh
yanu-cli pack --controlnca './base+update.xxxxxx/patchdata/control.nca' --titleid 'xxxxxxxxxxxxxxxx' --romfsdir './base+update.xxxxxx/romfs' --exefsdir './base+update.xxxxxx/exefs'
```
If only base was unpacked, get the control NCA from `basedata`.

> **Note**
> - For Windows, adapt the above examples by replacing `/` with `\` and using the appropriate path to the executable.
> - Control NCA is typically around 1MB in size.
> - Yanu only accepts Control Type NCA. If unsure of the Type, trial and error can help narrow down the options.
> - Check the logs for guidance on which TitleID to use if using the wrong one.

## Directories Used

| Used for | Windows                                  | Linux                                   |
| -------- | ---------------------------------------- | --------------------------------------- |
| Keys     | `%USERPROFILE%\.switch`                  | `$HOME/.switch`                         |
| Cache    | `%LOCALAPPDATA%\com.github.nozwock.yanu` | `$HOME/.cache/com.github.nozwock.yanu`  |
| Config   | `%APPDATA%\com.github.nozwock.yanu`      | `$HOME/.config/com.github.nozwock.yanu` |

## Troubleshooting

- Newer games failing to update.\
   Before applying the update, it is necessary to either use the latest product keys or perform a downgrade of the base and update the ROM files firmware using [NSCB](https://github.com/julesontheroad/NSC_BUILDER). This approach has proven to be effective.

- If you encounter errors such as 'Failed to sync package repos' while installing yanu on Termux, try updating your outdated repositories using:
   ```sh
   termux-change-repo
   ```

## Special Thanks

- [hactool](https://github.com/SciresM/hactool), [hacPack](https://github.com/The-4n/hacPack), [hac2l](https://github.com/Atmosphere-NX/hac2l), [hactoolnet](https://github.com/Thealexbarney/LibHac), and [4NXCI](https://github.com/The-4n/4NXCI) are used internally for the heavy lifting.
- [@Pipetto-crypto](https://github.com/Pipetto-crypto) for the `aarch64-linux` `hac2l` binary.
- [Willfaust's script](https://gist.github.com/willfaust/fb90dec409b8918290012031f09a78ef) for reference.
