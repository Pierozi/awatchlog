FROM ubuntu:xenial

RUN apt-get update && apt-get install -y \
      openssl \
      ca-certificates \
      curl \
      vim

# Installing rust
RUN curl https://sh.rustup.rs -sSf > /tmp/rust-installer \
      && chmod +x /tmp/rust-installer \
      && /tmp/rust-installer -y \
      && ln -s /root/.cargo/bin/cargo /usr/local/bin/cargo

# Script to push logs in local file
COPY write /usr/local/bin/write

COPY run.sh /run.sh
ENTRYPOINT ["/bin/bash", "/run.sh"]

CMD ["/usr/local/bin/write", "/var/log/my-app.log"]
