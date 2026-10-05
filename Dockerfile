FROM adregistry.fnal.gov/dev-containers/redhat-ubi9-minimal:9.8-1790754119

RUN useradd -u 10001 -r -M -s /sbin/nologin appuser

COPY --chown=10001:10001 target/release/grpc-alarms-db /usr/local/bin/grpc-alarms-db

USER 10001

ENTRYPOINT ["/usr/local/bin/grpc-alarms-db"]
