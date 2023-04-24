FROM python:3.9.6

SHELL ["/bin/bash", "--login", "-c"]

WORKDIR /usr/catalyst-relayer

ENV NODE_VERSION=18.16.0
RUN apt install -y curl
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
ENV NVM_DIR=/root/.nvm
RUN . "$NVM_DIR/nvm.sh" && nvm install ${NODE_VERSION}
RUN . "$NVM_DIR/nvm.sh" && nvm use v${NODE_VERSION}
RUN . "$NVM_DIR/nvm.sh" && nvm alias default v${NODE_VERSION}
ENV PATH="/root/.nvm/versions/node/v${NODE_VERSION}/bin/:${PATH}"

COPY package.json poetry.lock pyproject.toml package-lock.json ./
COPY scripts/testnet/* ./

RUN nvm install 18 && nvm use 18
RUN npm install

RUN python3 -m pip install --user pipx
RUN python3 -m pipx ensurepath
RUN pipx install poetry
RUN poetry install
ENV PATH="${PATH}:/root/.local/bin"

CMD ["poetry", "run", "python", "-u", "proxy_relayer.py"]
