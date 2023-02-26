<div align="center">

# yanu
Yet Another NSP Updater for [Skyline](https://github.com/skyline-emu/skyline) [**WIP**]

**Consider starring 🌟 the project if it helped**

</div align="center">

---

## About

**Yanu** is a **Nintendo Switch** rom patcher made to be compatible with Skyline. The supported platforms currently consist of **Linux**, **Android w/Termux** and **Windows**.

The software is to act as sort-of a band-aid until Skyline supports Updates/DLCs natively.

> **Note:** doesn't support DLCs yet.

Precompiled binaries are available from the [GitHub releases page](https://github.com/nozwock/yanu/releases).

## Features
- [x] NSPs updates
- [ ] XCIs updates
- [ ] NSZs updates

> **Note:** I might or might not add the features marked as unticked above.

## Installation

### Android w/Termux

- Copy-pasta
  ```console
  pkg install clang make binutils git -y && curl -sLo "$PATH/yanu" https://github.com/nozwock/yanu/releases/latest/download/yanu-aarch64-linux-android && chmod +x "$PATH/yanu" && termux-setup-storage
  ```

> Run by typing `yanu`

### Linux

1. Make sure dependencies required to build hactool/hacPack are installed
   ```console
   git gcc make binutils
   ```
2. Give executable permision from console or your file manager.
   ```console
   chmod +x yanu-x86_64-unknown-linux-musl
   ```

### Windows

> idk, AVs are choking `yanu` on windows rn 😵‍💫

## Credits

[hactool](https://github.com/SciresM/hactool), [hacPack](https://github.com/The-4n/hacPack)
