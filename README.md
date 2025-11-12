# gRPC Access to Persistent Data for Alarms

This service provides access to persistent data in the context of accelerator alarms. 

## Table of Contents
- [Supported endpoints](#supported-endpoints)
- [Sustainability](#sustainability)
  - [Development note](#development-note)
- [Rust docs](#rust-docs)

## Supported endpoints

Currently, the following methods are provided via gRPC. Full schema descriptions can be found in the `service/grpc-alarms-db/*` directory of the [`interface-definitions`](https://github.com/fermi-ad/interface-definitions) repository. That repository is a GitHub submodule of this project:
- `getGroupMetadata() -> AlarmGroupMetadata`
  - This method takes no parameters and returns all alarm group metadata in the database. 
  - Metadata consists of the group name, description, time of last update, user who made last update, and whether the group is also a category on the alarm screen.
- `getGroups(GroupsRequest) -> AlarmGroups`
  - This method takes a request parameter, containing a list of group names to retrieve.
  - Returns the requested group objects with their descendant groups and devices.
- `getUserLayouts() -> UserLayouts`
  - Method that takes no parameters and returns all user layouts. 
  - A `UserLayout` is the set of all top-level groups that a specific user has configured for their alarm screen. If a device in the group or a subgroup goes into alarm, it will appear in a category with the top-level group's name. 

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