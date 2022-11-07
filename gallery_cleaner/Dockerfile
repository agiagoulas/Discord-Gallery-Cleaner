# Builder
FROM rust:latest AS builder
RUN update-ca-certificates
WORKDIR /gallery_cleaner
COPY ./ .
RUN cargo build --release

# Image
FROM ubuntu:latest
WORKDIR /gallery_cleaner
COPY --from=builder /gallery_cleaner/target/release/gallery_cleaner ./
CMD ["/gallery_cleaner/gallery_cleaner"]
