# Sketch only. This file was not built in this workspace and no image was published.
# The binary refuses every non-loopback bind, so this is not a public relay image.
FROM rust:1.91
WORKDIR /src
COPY . .
RUN cargo build --release -p cbdesk-relayd
CMD ["cbdesk-relayd"]
