FROM ubuntu:lunar

WORKDIR /work

RUN apt update
RUN apt -y install gcc-12 g++-12 curl make git libjpeg-dev binutils-dev libicu-dev

RUN ln -s /usr/bin/g++-12 /usr/bin/g++
RUN ln -s /usr/bin/gcc-12 /usr/bin/gcc
RUN ln -s /usr/bin/gcc-12 /usr/bin/cc
RUN ln -s /usr/bin/gcc-ar-12 /usr/bin/gcc-ar
RUN ln -s /usr/bin/gcc-nm-12 /usr/bin/gcc-nm

RUN curl https://sh.rustup.rs | sh -s -- -y

COPY . .
WORKDIR /work/yanu-cli
RUN $HOME/.cargo/bin/cargo build --release
WORKDIR /work
RUN mv target/release/yanu-cli /usr/bin/yanu
RUN yanu build-backend