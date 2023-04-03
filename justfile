set shell := ["bash", "-uc"]

_default:
    @just --list

bin := file_name(invocation_directory())
target-android := "aarch64-unknown-linux-musl"
target-linux64 := "x86_64-unknown-linux-musl"
target-win64 := "x86_64-pc-windows-msvc"
target-win64-gnu := "x86_64-pc-windows-gnu"

android:
    cross build --target {{ target-android }} --release --features=android-proot
    @mv -T {{justfile_directory()}}/target/{{ target-android }}/release/{{ bin }} {{justfile_directory()}}/target/{{ bin }}-{{ replace(target-android, "unknown", "termux_proot") }}

linux64:
    cross build --target {{ target-linux64 }} --release
    @mv -T {{justfile_directory()}}/target/{{ target-linux64 }}/release/{{ bin }} {{justfile_directory()}}/target/{{ bin }}-{{ target-linux64 }}

win64:
    cargo build --target {{ target-win64 }} --release
    @mv -t {{justfile_directory()}}/target/{{ target-win64 }}/release/{{ bin }}.exe {{justfile_directory()}}/target/{{ bin }}-{{ target-win64 }}.exe

win64-gnu:
    cross build --target {{ target-win64-gnu }} --release
    @mv -t {{justfile_directory()}}/target/{{ target-win64-gnu }}/release/{{ bin }}.exe {{justfile_directory()}}/target/{{ bin }}-{{ target-win64-gnu }}.exe