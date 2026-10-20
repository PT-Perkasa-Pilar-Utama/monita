# The release image: the static Linux binary that the release workflow built and booted, on a
# base that holds CA certificates, time zones and a non-root user (uid 65532), and no shell.
#
# It does not build from source. The release workflow prepares the context:
#   linux_amd64/monita  linux_arm64/monita  LICENSE  licenses/  data/ (empty)
# and builds both platforms at once with `docker buildx build -f deploy/Dockerfile <context>`.
FROM gcr.io/distroless/static-debian12:nonroot@sha256:afa5c872c891853ca7fcf1f12c3edb23f7eeef36189728842dd51042ff57f7ab

ARG TARGETARCH
COPY linux_${TARGETARCH}/monita /usr/local/bin/monita
COPY LICENSE /usr/share/monita/LICENSE
COPY licenses /usr/share/monita/licenses
# A named volume takes this folder's owner, so the non-root server can write its database.
COPY --chown=65532:65532 data /data

# prod refuses an http:// MONITA_BASE_URL. To try it on a laptop, pass -e MONITA_ENV=dev.
ENV MONITA_ENV=prod \
    MONITA_BIND=0.0.0.0:7466 \
    MONITA_DB_PATH=/data/monita.db
VOLUME /data
EXPOSE 7466

LABEL org.opencontainers.image.source="https://github.com/PT-Perkasa-Pilar-Utama/monita" \
      org.opencontainers.image.title="Monita" \
      org.opencontainers.image.description="Self-hosted uptime, incident, status-page and bug-ticket service" \
      org.opencontainers.image.licenses="MIT"

ENTRYPOINT ["/usr/local/bin/monita"]
CMD ["serve"]
