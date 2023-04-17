FROM ubuntu:lunar as build

WORKDIR /work

RUN apt update \
    && apt -y install gcc-12 g++-12 curl make git libjpeg-dev binutils-dev libicu-dev

RUN ln -s /usr/bin/g++-12 /usr/bin/g++
RUN ln -s /usr/bin/gcc-12 /usr/bin/gcc
RUN ln -s /usr/bin/gcc-12 /usr/bin/cc
RUN ln -s /usr/bin/gcc-ar-12 /usr/bin/gcc-ar
RUN ln -s /usr/bin/gcc-nm-12 /usr/bin/gcc-nm

RUN curl https://sh.rustup.rs | sh -s -- -y

COPY . .
RUN $HOME/.cargo/bin/cargo build --release --bin yanu-cli
RUN mv target/release/yanu-cli /usr/bin/yanu
RUN yanu setup-backend

FROM ubuntu:lunar as runtime

WORKDIR /work
COPY --from=build /root/.cache /root/.cache
COPY --from=build /root/.config /root/.config
COPY --from=build /usr/bin/yanu /usr/bin/yanu

RUN apt update \
    && apt -y install libicu-dev \
    && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["yanu"]