FROM adregistry.fnal.gov/dev-containers/redhat-ubi9-minimal

COPY target/release/grpc-alarms-db /app/grpc-alarms-db 

WORKDIR /app

ENV ALARM_GRPC_SERVER_PORT=7055
EXPOSE 7055

CMD ["./grpc-alarms-db"]