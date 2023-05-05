set shell := ["bash", "-uc"]

_default:
    @just --list

bin := "yanu"
bin-cli := "yanu-cli"
target-android := "aarch64-unknown-linux-musl"
target-linux64 := "x86_64-unknown-linux-musl"
target-linux64-gnu := "x86_64-unknown-linux-gnu"
target-win64 := "x86_64-pc-windows-msvc"
target-win64-gnu := "x86_64-pc-windows-gnu"

android:
    cross build --target {{ target-android }} --release --features=android-proot --bin {{ bin-cli }}

    mkdir -p {{justfile_directory()}}/dist
    @mv -T {{justfile_directory()}}/target/{{ target-android }}/release/{{ bin-cli }} {{justfile_directory()}}/dist/{{ bin-cli }}-{{ replace(target-android, "unknown", "termux_proot") }}

linux64:
    cross build --target {{ target-linux64-gnu }} --release --bin {{ bin }}
    cross build --target {{ target-linux64 }} --release --bin {{ bin-cli }}

    mkdir -p {{justfile_directory()}}/dist
    @mv -T {{justfile_directory()}}/target/{{ target-linux64-gnu }}/release/{{ bin }} {{justfile_directory()}}/dist/{{ bin }}-{{ target-linux64-gnu }}
    @mv -T {{justfile_directory()}}/target/{{ target-linux64 }}/release/{{ bin-cli }} {{justfile_directory()}}/dist/{{ bin-cli }}-{{ target-linux64 }}

win64:
    cargo build --target {{ target-win64 }} --release

    @mv -T {{justfile_directory()}}/target/{{ target-win64 }}/release/{{ bin }}.exe {{justfile_directory()}}/target/{{ bin }}-{{ target-win64 }}.exe
    @mv -T {{justfile_directory()}}/target/{{ target-win64 }}/release/{{ bin-cli }}.exe {{justfile_directory()}}/target/{{ bin-cli }}-{{ target-win64 }}.exe

win64-gnu:
    cross build --target {{ target-win64-gnu }} --release

    @mv -T {{justfile_directory()}}/target/{{ target-win64-gnu }}/release/{{ bin }}.exe {{justfile_directory()}}/target/{{ bin }}-{{ target-win64-gnu }}.exe
    @mv -T {{justfile_directory()}}/target/{{ target-win64-gnu }}/release/{{ bin-cli }}.exe {{justfile_directory()}}/target/{{ bin-cli }}-{{ target-win64-gnu }}.exe