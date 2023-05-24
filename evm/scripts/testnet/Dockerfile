FROM python:3.9.6

SHELL ["/bin/bash", "--login", "-c"]

WORKDIR /usr/catalyst-relayer

COPY poetry.lock pyproject.toml ./
COPY scripts/deploy_config.json ./
COPY scripts/testnet/* ./

RUN python3 -m pip install --user pipx
RUN python3 -m pipx ensurepath
RUN pipx install poetry
RUN poetry install
ENV PATH="${PATH}:/root/.local/bin"

CMD ["poetry", "run", "python", "-u", "poa_relayer.py", "deploy_config.json"]