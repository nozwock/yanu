<div align="center">

# yanu
Yet Another NSP Updater for [Skyline](https://github.com/skyline-emu/skyline)

**Consider starring 🌟 the project if it helped**

</div align="center">

---

## About

**Yanu** is a **Nintendo Switch** ROM updater made to be compatible with Skyline. The supported platforms currently consist of **Android w/Termux**, **Linux** and **Windows**.

The software is to act as sort-of a band-aid until Skyline supports Updates/DLCs natively.

> **Warning** - **Doesn't support DLCs.**

Precompiled binaries are available from the [GitHub releases page](https://github.com/nozwock/yanu/releases).

### Supported File Types
- [x] NSP 
- [ ] XCI

> **Note** - Support for others might be added.

## Installation

### Android w/Termux

> **Check out [Video Tutorial](https://www.youtube.com/watch?v=rsYHWL7G3EI) by Zerokimchi for Android.** _(outdated command in the video)_

1. Download & Install `Termux` from [F-droid](https://f-droid.org/en/packages/com.termux/).
2. Copy-paste the following in `Termux` and hit enter:
  ```console
  bash <(curl -L https://raw.githubusercontent.com/nozwock/yanu/main/scripts/yanu-bootstrap-termux.sh)
  ```

Relevant resources:
- MiXplorer [XDA Forum](https://forum.xda-developers.com/t/app-2-2-mixplorer-v6-x-released-fully-featured-file-manager.1523691/)
- MiXplorer GDrive [download link](https://drive.google.com/drive/folders/1BfeK39boriHy-9q76eXLLqbCwfV17-Gv)


### Linux

1. Make sure dependencies required to build hactool/hacPack are installed on your system, for eg-
   ```console
   sudo apt -y install gcc-12 g++-12 make git libjpeg-dev binutils-dev libicu-dev
   ```
2. Download & give executable permission to `yanu`:
   ```console
   chmod +x yanu-x86_64-unknown-linux-musl
   ```

### Windows

- Just [download](https://github.com/nozwock/yanu/releases) & run.

> **Note**\
> Since the builds are not code-signed, some AVs might false positively flag the program as malicious.\
> Not much can be done about it since I'm not willing to pay for those expensive certs.\
> Use these with common sense if you're paranoid ig :weary: -
> - https://www.virustotal.com
> - https://www.hybrid-analysis.com 

### Docker
Go [here](https://github.com/nozwock/yanu/pkgs/container/yanu) and pull the container you'd like to use.
Then it can be used as such:
   ```sh
      # Expecting 'prod.keys` in pwd
      docker run -v $(pwd)/prod.keys:/root/.switch/prod.keys -v $(pwd):/work ghcr.io/nozwock/yanu update --base '/path/to/base' --update '/path/to/update' 
   ```

## Usage (CLI only)
View CLI help with:
```sh
yanu --help
```

For updating a ROM:
```sh
yanu --keyfile '/path/to/keyfile' update --base '/path/to/base' --update '/path/to/update'
```

Set a new `Roms Directory` (Used in `update-prompt`) with:
```sh
yanu config --roms-dir '/new/path/here'
```

For unpacking ROMs:
```sh
yanu unpack --base '/path/to/base' --update '/path/to/update'
```

OR, for only unpacking a single ROM:
```sh
yanu unpack --base '/path/to/base'
```

For repacking unpacked ROM data:
```sh
yanu repack --controlnca './base+update.bylies/patchdata/123456.nca' --romfsdir './base+update.bylies/romfs' --exefsdir './base+update.bylies/exefs'
```

> **Note**
> - The above examples were for *nix systems, adapt them appropriately for Windows (Replacing `/` with `\` and with proper path to the executable).
> - Control NCA is usually the NCA around ~1MiB in size.
> - Yanu will only accept Control Type NCA, so you can atleast figure out the Type by trial & error incase it's too hard to guess.

## Directories Used

| Used for | Windows | Linux |
| --- | --- | --- |
| Keys | `%USERPROFILE%\.switch` | `$HOME/.switch` |
| Cache | `%LOCALAPPDATA%\com.github.nozwock.yanu` | `$HOME/.cache/com.github.nozwock.yanu` |
| Config | `%APPDATA%\com.github.nozwock.yanu` | `$HOME/.config/com.github.nozwock.yanu` |

## Troubleshooting

- For "Failed to sync package repos" like errors while trying to install `yanu` on Termux:
   - Update your outdated repos using:
      ```sh
      termux-change-repo
      ```

## Special Thanks

- [hactool](https://github.com/SciresM/hactool), [hacPack](https://github.com/The-4n/hacPack), [hac2l](https://github.com/Atmosphere-NX/hac2l) and [hactoolnet](https://github.com/Thealexbarney/LibHac) used internally for the heavy lifting.
- [@Pipetto-crypto](https://github.com/Pipetto-crypto) for the `aarch64-linux` `hac2l` binary.
- [Willfaust's script](https://gist.github.com/willfaust/fb90dec409b8918290012031f09a78ef) for reference.
