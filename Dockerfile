FROM debian:bookworm-slim

COPY target/release/grpc-alarms-db /app/grpc-alarms-db 

# Pull in the file with the basic environment variables.
# At deployment, the container will need the following additional variables passed:
#    - PGUSER => The database account used by the app to talk to the Postgres DB.
#    - PGPASSWORD or PGPASSFILE (and the associated file provided to the container)
#        => The credentials needed to authenticate as the specified DB access user.
COPY .env /app/.env

WORKDIR /app
EXPOSE 7055
CMD ["./grpc-alarms-db"]