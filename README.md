# gRPC Access to Persistent Data for Alarms

A Rust-based gRPC microservice that provides structured access to persistent alarm data for accelerator control systems at Fermilab. It abstracts all database interaction behind a well-defined Protobuf interface, so consuming services remain unaware of the underlying storage implementation.

## Table of Contents

- [Architecture](#architecture)
- [Services](#services)
  - [Alarm Groups](#alarm-groups)
  - [Alarm Timers](#alarm-timers)
  - [User Layouts](#user-layouts)
- [Database Schema](#database-schema)
- [Environment Variables](#environment-variables)
- [Running the Service](#running-the-service)
  - [CI/CD Pipelines](#cicd-pipelines)
  - [Docker Image (CI/CD managed)](#docker-image-cicd-managed)
  - [Local Development](#local-development)
- [Testing](#testing)
- [Project Structure](#project-structure)
- [Dependencies](#dependencies)
- [Sustainability](#sustainability)
  - [Development Note](#development-note)
- [Rust Docs](#rust-docs)

---

## Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                        gRPC Clients                                │
│              (Alarm Display, Control System Services, etc.)        │
└───────────────────────────┬────────────────────────────────────────┘
                            │  gRPC (see Dockerfile for port)
                            │  Protobuf-defined interface
                            ▼
┌────────────────────────────────────────────────────────────────────┐
│                     grpc-alarms-db Service                         │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                      Tonic gRPC Server                      │   │
│  │                                                             │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐  │   │
│  │  │  AlarmGroup     │  │  AlarmTimer     │  │ UserLayouts │  │   │
│  │  │  Service        │  │  Service        │  │ Service     │  │   │
│  │  └────────┬────────┘  └────────┬────────┘  └──────┬──────┘  │   │
│  │           │                    │                  │         │   │
│  │           └────────────────────┴──────────────────┘         │   │
│  │                                │                            │   │
│  │                    ┌───────────▼──────────┐                 │   │
│  │                    │   DataStore Trait    │                 │   │
│  │                    │   (rust-db-lib)      │                 │   │
│  │                    └───────────┬──────────┘                 │   │
│  └────────────────────────────────┼────────────────────────────┘   │
│                                   │                                │
│  ┌────────────────────────────────▼────────────────────────────┐   │
│  │              PostgresDataStore (rust-db-lib)                │   │
│  └────────────────────────────────┬────────────────────────────┘   │
└───────────────────────────────────┼────────────────────────────────┘
                                    │  SQL (PostgreSQL)
                                    ▼
┌────────────────────────────────────────────────────────────────────┐
│                     PostgreSQL Database                            │
│                     schema: alarmsapp                              │
│                                                                    │
│   ┌──────────────┐   ┌──────────────────┐   ┌──────────────────┐   │
│   │    groups    │   │ group_membership │   │  user_layouts    │   │
│   └──────┬───────┘   └────────┬─────────┘   └──────────────────┘   │
│          │ (see app_schema.sql for full column definitions)        │
│   ┌──────┴───────┐   ┌────────┴─────────┐                          │
│   │ timer_types  │◄──│     timers       │                          │
│   └──────────────┘   └──────────────────┘                          │
└────────────────────────────────────────────────────────────────────┘
```

The service is intentionally structured so that **only one layer** (`PostgresDataStore` from `rust-db-lib`) knows the database implementation detail. All three gRPC service implementations depend on the abstract `DataStore` trait, making the storage backend swappable without touching service logic.

---

## Services

The full Protobuf schema definitions — including all RPC method signatures, request/response message types, and field descriptions — are maintained in the [interface-definitions repository](https://github.com/fermi-ad/interface-definitions/tree/main/proto/controls/service/grpc-alarms-db/v1). That repository is the authoritative source for the gRPC contract; refer to it for up-to-date method and message details.

### Alarm Groups

**Proto service:** `AlarmGroupService`

Provides read access to hierarchical alarm group definitions. Groups can contain devices or other groups (nested), and the service uses a recursive SQL CTE to resolve the full membership tree.

### Alarm Timers

**Proto service:** `AlarmTimerService`

Manages alarm timers (e.g. snooze and bypass reminder). Timers are associated with a device and have an expiry time. Supports create, read, update, and delete operations. All input is validated before hitting the database; device names and usernames are normalised to lowercase.

### User Layouts

**Proto service:** `UserLayoutsService`

Stores and retrieves each user's personalised alarm screen layout — specifically, which top-level alarm groups appear as categories on their alarm display.

---

## Database Schema

The service operates against a PostgreSQL database using the `alarmsapp` schema. The full DDL — including all table definitions, column types, constraints, and triggers — is in [`resources/sql/app_schema.sql`](resources/sql/app_schema.sql). That file is the authoritative source of truth for the schema; the summary below is for orientation only.

| Table | Purpose |
|---|---|
| `alarmsapp.groups` | Master list of alarm groups with metadata |
| `alarmsapp.group_membership` | Many-to-many membership: which devices/groups belong to which group |
| `alarmsapp.user_layouts` | Per-user list of top-level alarm group categories |
| `alarmsapp.timer_types` | Lookup table for timer type names |
| `alarmsapp.timers` | Active alarm timers keyed by device and timer type |

All tables include `updated_at` (auto-set via trigger) and `updated_by` audit columns.

A data migration script is also available at [`resources/sql/data_cutover.sql`](resources/sql/data_cutover.sql).

---

## Environment Variables

The following variables must be set for the service to run:

| Variable | Required | Description |
|---|---|---|
| `DATABASE_HOST` | ✅ | Hostname or IP of the PostgreSQL server |
| `DATABASE_PORT` | ✅ | Port for the database connection. Must be a valid `u16` |
| `DATABASE_USER` | ✅ | PostgreSQL username |
| `DATABASE_PASS` | ✅ | Password for the PostgreSQL user |
| `DATABASE_NAME` | ✅ | Name of the target database |
| `ALARM_GRPC_SERVER_PORT` | ✅ | Port the gRPC server listens on (see [`Dockerfile`](Dockerfile) for the default) |
| `RUST_LOG` | ❌ | Controls log verbosity (e.g. `info`, `debug`, `grpc_alarms_db=trace`). See [tracing-subscriber docs](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) |

---

## Running the Service

> **Packaging and deployment are handled automatically by the CI/CD pipeline and downstream infrastructure services.** Developers do not need to build Docker images or deploy the service manually. Merge to the appropriate branch and the pipeline takes care of the rest.

### CI/CD Pipelines

Two GitHub Actions workflows govern the automated lifecycle of this service:

| Workflow | File | Purpose |
|---|---|---|
| Rust Continuous Integration | [`.github/workflows/integration.yaml`](.github/workflows/integration.yaml) | Runs tests and generates code coverage reports |
| Continuous Deployment | [`.github/workflows/deployment.yaml`](.github/workflows/deployment.yaml) | Builds the release binary, packages the Docker image, and deploys via downstream infrastructure |

Both workflows delegate to shared reusable workflows maintained in the `fermi-ad/.github` organization repository. Refer to the workflow files for current trigger conditions.

> **Note on coverage:** The `queries.rs` files (which contain only static SQL string constants inlined at compile time) and `services/mod.rs` (which contains only module declarations) are intentionally excluded from coverage reporting.

### Docker Image (CI/CD managed)

The [`Dockerfile`](Dockerfile) defines the production container image. It is built on a Red Hat UBI9 minimal base and packages the compiled release binary. The image is built, tagged, and pushed to the container registry by the CD pipeline — **not by developers directly**.

The server inside the container listens on all IPv6 interfaces (`[::]`) on the configured port.

The required environment variables (see [Environment Variables](#environment-variables)) are injected at runtime by the deployment infrastructure.

### Local Development

This repository ships with a [`devcontainer.json`](.devcontainer/devcontainer.json) that provides a fully configured Rust development environment. Install the **Dev Containers** extension in VS Code and reopen the project in the container when prompted.

```bash
# Run tests
cargo test

# Build (debug)
cargo build

# Build (release, with LTO and symbol stripping)
cargo build --release

# Run locally (requires environment variables to be set)
cargo run
```

---

## Testing

Each service module has a dedicated test file using an in-memory `TestDataStore` from `rust-db-lib`'s `testing-utils` feature. This allows full unit testing of service logic without a live database connection.

| Test file | Coverage |
|---|---|
| [`src/services/alarm_groups/tests.rs`](src/services/alarm_groups/tests.rs) | Group metadata retrieval, group detail queries, sorting |
| [`src/services/alarm_timers/tests.rs`](src/services/alarm_timers/tests.rs) | Timer CRUD, input validation, timer type routing |
| [`src/services/user_layouts/tests.rs`](src/services/user_layouts/tests.rs) | Layout retrieval and per-user grouping |
| [`src/logging/tests.rs`](src/logging/tests.rs) | Logging setup |
| [`src/tests.rs`](src/tests.rs) | Top-level integration tests |

Run all tests with:

```bash
cargo test
```

---

## Project Structure

```
grpc-alarms-db/
├── .github/
│   ├── dependabot.yml                # Automated dependency update schedule (Cargo, Actions, devcontainer)
│   └── workflows/
│       ├── integration.yaml          # CI: runs tests and coverage on push/PR to main
│       └── deployment.yaml           # CD: builds image and deploys on push to main
├── build.rs                          # Build script: compiles .proto files via rust-grpc-lib
├── Cargo.toml                        # Package manifest and dependency declarations
├── Dockerfile                        # Container image definition (built and pushed by CD pipeline)
├── resources/
│   └── sql/
│       ├── app_schema.sql            # PostgreSQL schema DDL for the alarmsapp schema
│       └── data_cutover.sql          # Data migration script
└── src/
    ├── main.rs                       # Entry point: wires up DB config, services, and gRPC server
    ├── proto.rs                      # Includes generated Protobuf/gRPC Rust code (from OUT_DIR)
    ├── utils.rs                      # Shared utilities (e.g. DateTime → Protobuf Timestamp)
    ├── tests.rs                      # Top-level integration tests
    ├── logging/
    │   ├── mod.rs                    # Logging setup using tracing + tracing-subscriber
    │   └── tests.rs
    └── services/
        ├── mod.rs                    # Re-exports all service modules
        ├── alarm_groups/
        │   ├── mod.rs                # AlarmGroupService gRPC implementation
        │   ├── queries.rs            # SQL queries for alarm groups
        │   └── tests.rs
        ├── alarm_timers/
        │   ├── mod.rs                # AlarmTimerService gRPC implementation + input validation
        │   ├── queries.rs            # SQL queries for alarm timers
        │   └── tests.rs
        └── user_layouts/
            ├── mod.rs                # UserLayoutsService gRPC implementation
            ├── queries.rs            # SQL queries for user layouts
            └── tests.rs
```

---

## Dependencies

[`Cargo.toml`](Cargo.toml) is the authoritative source for all dependencies and their pinned versions. The table below describes the role of each dependency for orientation.

| Crate | Purpose |
|---|---|
| [`tonic`](https://crates.io/crates/tonic) | gRPC server framework for Rust |
| [`tonic-health`](https://crates.io/crates/tonic-health) | gRPC health-check protocol implementation |
| [`prost`](https://crates.io/crates/prost) | Protobuf encoding/decoding |
| [`tokio`](https://crates.io/crates/tokio) | Async runtime (multi-thread) |
| [`chrono`](https://crates.io/crates/chrono) | Date/time handling |
| [`tracing`](https://crates.io/crates/tracing) + [`tracing-subscriber`](https://crates.io/crates/tracing-subscriber) | Structured logging |
| `rust-db-lib` (internal) | Abstract `DataStore` trait + `PostgresDataStore` implementation |
| `rust-env-var-lib` (internal) | Typed environment variable loading |
| `rust-grpc-lib` (internal) | Proto compilation build support |

Dependency updates are automated via [Dependabot](.github/dependabot.yml). See that file for current update schedules and grouping rules.

---

## Sustainability

This repository is architected with longevity in mind. Please do your part to keep it maintainable for the indefinite future:

- **Write tests.** Early, often, and comprehensively. Run them regularly.
- **Respect the abstraction boundary.** Only `rust-db-lib`'s `PostgresDataStore` knows the database implementation. Service logic depends only on the `DataStore` trait. This makes the storage backend swappable. Do not leak database-specific details into service modules.
- **Open issues** for anything and everything — even small nitpicks. Better to track things visibly than let them slip through the cracks.
- **Be professional.** Use clear names, write small methods, break logic into digestible pieces, and be a good steward of the system. The time you spend making it correct saves mountains of time in future maintenance.
- **Update this document.** If you find a pitfall or a lesson learned, put it here so others don't have to fight the same fires.

### Development Note

This repository ships with a `devcontainer.json` file referencing a prebuilt development container with all necessary Rust tooling. Install the **Dev Containers** extension in VS Code and you will be prompted to reopen the project in the container. This enforces consistent tool versions across developer machines and saves setup time.

---

## Rust Docs

The Rust documentation and getting-started guide can be found [here](https://doc.rust-lang.org/book/title-page.html).

Generate local API documentation for this crate with:

```bash
cargo doc --open
```
