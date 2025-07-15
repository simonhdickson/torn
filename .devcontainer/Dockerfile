FROM fedora:41

RUN dnf install -y @development-tools zsh systemd-devel

ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && dnf install -y sudo \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

USER $USERNAME

RUN mkdir -p /home/$USERNAME/.local/bin

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

ENV PATH=/home/$USERNAME/.cargo/bin:$PATH
