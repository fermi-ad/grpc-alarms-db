FROM debian:bookworm-slim

RUN apt-get update -y && apt-get install -y libssl3 && apt-get clean -y

COPY target/release/grpc-alarms-db /app/grpc-alarms-db 

WORKDIR /app
EXPOSE 7055
CMD ["./grpc-alarms-db"]