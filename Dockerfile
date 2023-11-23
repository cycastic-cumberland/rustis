FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

# Create appuser
ENV USER=rest_server
ENV UID=10001

RUN mkdir /home/rest_server

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/home/rest_server" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /home/rest_server

COPY ./ .

RUN cargo build --target x86_64-unknown-linux-musl --release
RUN strip -s /home/rest_server/target/x86_64-unknown-linux-musl/release/rustis

FROM scratch as final

EXPOSE 8288

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /home/rest_server

# Copy our build
COPY --from=builder /home/rest_server/target/x86_64-unknown-linux-musl/release/rustis ./

# Use an unprivileged user.
USER rest_server:rest_server

CMD ["/home/rest_server/rustis"]
