# gRPC Access to Persistent Data for Alarms

This service provides access to persistent data in the context of accelerator alarms. 

## Table of Contents
- [Supported endpoints](#supported-endpoints)
- [Environment Variables](#environment-variables)
- [Sustainability](#sustainability)
  - [Development note](#development-note)
- [Rust docs](#rust-docs)

## Supported endpoints

The full schema descriptions of the gRPC methods provided by this service can be found [here](https://github.com/fermi-ad/interface-definitions/tree/main/proto/controls/service/grpc-alarms-db/v1).

## Environment Variables
The following variables must be set for this service to run:
- `DATABASE_HOST` - The host of the database, e.g. `localhost`, `10.32.12.53`, `fermi-db.fnal.gov`, etc.
- `DATABASE_PORT` - The port to use on the host. Must parse to an unsigned 16-bit integer. If none is provided, the default port for PostgreSQL will be used (5432).
- `DATABASE_USER` - The username to use when connecting to the database.
- `DATABASE_PASS` - The password for the desired user.
- `DATABASE_NAME` - The name of the database being connected to, e.g. `adbs`

## Sustainability

This repository is architected with longevity in mind. Please do your part to keep it maintainable for the indefinite future. This includes:
- Write tests!!!! Early, often, and comprehensively. Run those tests regularly. 
- Pay attention to structure and organization. This repo has been built to abstract details and separate concerns. As of this writing, this service interacts with a PostgreSQL database. There is only one module of the project that knows this detail. This allows the choice of database to be changed quickly, should the data be migrated somewhere else in the future. Please do your part to incorporate this ethos into all changes you make.
- Open issues - for anything and everything. Even small nitpicks. Better to see things in an observable way than let them slip through the cracks.
- Be professional. This means using clear names, writing small methods, breaking out logic into digestible pieces, and generally being a good steward of the system. The little bit of time you spend making it correct saves mountains of time down the road in maintenance. Pay it forward. Be kind to future maintainers, including  your future self.
- Update this document. If you find a pitfall or a lesson learned, put it here so others don't have to fight the same fires.

### Development note
This repository comes with a `devcontainer.json` file, which references a prebuilt development container that should have all the necessary tools for developing in Rust. Please make use of this. Install the "Dev Containers" extension in VS Code and you should be prompted to reopen the project in the container. This will save you the headache of having to install things yourself, and will enforce tool versions across different developer machines. 

## Rust docs
The Rust documentation and a getting-started guide can be found [here](https://doc.rust-lang.org/book/title-page.html).