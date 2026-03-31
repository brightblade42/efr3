FROM registry.fedoraproject.org/fedora:latest

LABEL description="DevTools: Rust, Go, Deno, Nushell, LazyVim, and multiplexers."

# 1. Distrobox Core, Build Basics, Editor, and SQLite
RUN dnf update -y && dnf install -y \
    sudo passwd procps-ng findutils util-linux \
    curl wget git tar xz unzip zip \
    gcc gcc-c++ openssl-devel pkg-config \
    neovim tmux sqlite \
    # Essential CLI & LazyVim Dependencies
    ripgrep fd-find bat fzf zoxide jq btop \
    && dnf clean all


# 2. Mise & Core Languages (Baked into /opt)
ENV MISE_DATA_DIR=/opt/mise
ENV PATH="/opt/mise/shims:$PATH"

RUN dnf install -y dnf-plugins-core && \
    dnf config-manager addrepo --from-repofile=https://mise.jdx.dev/rpm/mise.repo && \
    dnf install -y mise && \
    # Install Runtimes
    mise use --global rust@latest go@latest deno@latest && \
    mise install -y && \
    # Install Bacon ONLY via Cargo
    mise exec rust -- cargo install bacon && \
    cp /root/.cargo/bin/bacon /usr/local/bin/bacon && \
    # Cleanup to shrink image size
    rm -rf /opt/mise/downloads/* /opt/mise/cache/* /root/.cargo

# 3. Pre-compiled Modern Binaries (QSV, Zellij, & Nushell)
RUN set -e; \
    # QSV (Pre-compiled to avoid Rust crate dependency issues)
    QSV_VER=$(curl -s https://api.github.com/repos/dathere/qsv/releases/latest | grep '"tag_name"' | sed -E 's/.*"v?([^"]+)".*/\1/') && \
    curl -L "https://github.com/dathere/qsv/releases/download/${QSV_VER}/qsv-${QSV_VER}-x86_64-unknown-linux-gnu.zip" -o /tmp/qsv.zip && \
    unzip /tmp/qsv.zip qsv -d /usr/local/bin/ && \
    chmod +x /usr/local/bin/qsv && \
    rm /tmp/qsv.zip && \
    # Zellij
    ZELLIJ_VER=$(curl -s https://api.github.com/repos/zellij-org/zellij/releases/latest | grep '"tag_name"' | sed -E 's/.*"v?([^"]+)".*/\1/') && \
    curl -L "https://github.com/zellij-org/zellij/releases/download/v${ZELLIJ_VER}/zellij-x86_64-unknown-linux-musl.tar.gz" | tar -xz -C /usr/local/bin && \
    chmod +x /usr/local/bin/zellij && \
    # Nushell
    NU_VER=$(curl -s https://api.github.com/repos/nushell/nushell/releases/latest | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/') && \
    curl -L "https://github.com/nushell/nushell/releases/download/${NU_VER}/nu-${NU_VER}-x86_64-unknown-linux-musl.tar.gz" | tar -xz -C /usr/local/bin --strip-components=1 --wildcards '*/nu' '*/nu_plugin_*' && \
    chmod +x /usr/local/bin/nu

# 4. LazyVim Staging (The Safety Net)
RUN git clone https://github.com/LazyVim/starter /opt/lazyvim && \
    rm -rf /opt/lazyvim/.git


# 5. Global Shell Automation (bashrc)
RUN echo 'eval "$(mise activate bash)"' >> /etc/bashrc && \
    echo 'eval "$(zoxide init bash)"' >> /etc/bashrc && \
    echo "alias podman='distrobox-host-exec podman'" >> /etc/bashrc && \
    echo "alias docker='distrobox-host-exec docker'" >> /etc/bashrc && \
    echo "alias cat='bat --paging=never'" >> /etc/bashrc && \
    echo "alias ls='ls --color=auto'" >> /etc/bashrc && \
    echo "alias ll='ls -la --color=auto'" >> /etc/bashrc && \
    echo "alias xsv='qsv'" >> /etc/bashrc && \
    echo '[ ! -d ~/.config/nvim ] && cp -r /opt/lazyvim ~/.config/nvim && echo "LazyVim Staged."' >> /etc/bashrc

# Ensure Distrobox can map your user to root seamlessly
RUN echo "%wheel ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/wheel

ENV EDITOR=nvim
CMD ["/bin/bash"]
