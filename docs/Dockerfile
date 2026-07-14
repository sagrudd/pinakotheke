FROM python:3.13.5-slim-bookworm@sha256:4c2cf9917bd1cbacc5e9b07320025bdb7cdf2df7b0ceaccb55e9dd7e30987419

ENV PIP_DISABLE_PIP_VERSION_CHECK=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

WORKDIR /workspace
COPY docs/requirements.txt /tmp/x-img-docs-requirements.txt
RUN python -m pip install --no-cache-dir -r /tmp/x-img-docs-requirements.txt
COPY docs /workspace/docs

RUN sphinx-build -W -b html /workspace/docs /workspace/docs/_build/html

CMD ["sphinx-build", "-W", "-b", "dummy", "/workspace/docs", "/tmp/x-img-docs-check"]
