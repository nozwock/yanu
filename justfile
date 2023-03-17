set shell := ["bash", "-uc"]

_default:
    @just --list

bin := "yanu"
target-android := "aarch64-unknown-linux-musl"
target-linux64 := "x86_64-unknown-linux-musl"
target-win64 := "x86_64-pc-windows-msvc"
target-win64-gnu := "x86_64-pc-windows-gnu"

android:
    cross build --target {{ target-android }} --release --features=android-proot
    @mv -T ./target/{{ target-android }}/release/{{ bin }} ./target/{{ bin }}-{{ replace(target-android, "unknown", "termux_proot") }}

linux64:
    cross build --target {{ target-linux64 }} --release
    @mv -T ./target/{{ target-linux64 }}/release/{{ bin }} ./target/{{ bin }}-{{ target-linux64 }}

win64:
    cargo build --target {{ target-win64 }} --release
    @mv -t ./target/{{ target-win64 }}/release/{{ bin }}.exe ./target/{{ bin }}-{{ target-win64 }}.exe

win64-gnu:
    cross build --target {{ target-win64-gnu }} --release
    @mv -t ./target/{{ target-win64-gnu }}/release/{{ bin }}.exe ./target/{{ bin }}-{{ target-win64-gnu }}.exe