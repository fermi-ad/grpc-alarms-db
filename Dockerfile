FROM debian:bookworm-slim

COPY target/release/grpc-alarms-db /app/grpc-alarms-db 

WORKDIR /app
EXPOSE 7055
CMD ["./grpc-alarms-db"]