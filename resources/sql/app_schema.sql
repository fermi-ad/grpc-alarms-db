/*
 * Script to generate the tables for the grpc-alarms-db service
 */

-- Create the trigger function for automatically setting the updated_at field in the tables. --
CREATE OR REPLACE FUNCTION alarmsapp.set_updated_at()
RETURNS TRIGGER AS 
$$
BEGIN
    NEW.updated_at := NOW();
	RETURN NEW;
END;
$$
LANGUAGE plpgsql;

-- Generate the "parent" table that knows all the groups. --
CREATE TABLE alarmsapp.groups ( 
    group_name VARCHAR(250) PRIMARY KEY,
	description TEXT,
	updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
	updated_by CITEXT NOT NULL 
);

CREATE TRIGGER set_updated_at_trigger
BEFORE INSERT OR UPDATE ON alarmsapp.groups 
FOR EACH ROW EXECUTE PROCEDURE alarmsapp.set_updated_at();

-- Generate the table that knows which groups and devices are inside a group. --
CREATE TABLE alarmsapp.group_membership (
    group_name VARCHAR(250),
	member_name VARCHAR(250),
	member_is_group BOOLEAN NOT NULL,
	updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
	updated_by CITEXT NOT NULL,
	PRIMARY KEY (group_name, member_name),
	FOREIGN KEY (group_name) REFERENCES alarmsapp.groups(group_name)
);

CREATE TRIGGER set_updated_at_trigger
BEFORE INSERT OR UPDATE ON alarmsapp.group_membership 
FOR EACH ROW EXECUTE PROCEDURE alarmsapp.set_updated_at();


-- Generate the table that knows all the top-level groups for a user (displayed on the alarm screen as categories). --
CREATE TABLE alarmsapp.user_layouts (
    user_name CITEXT,
	group_name VARCHAR(250),
	updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
	updated_by CITEXT NOT NULL,
	PRIMARY KEY (user_name, group_name),
	FOREIGN KEY (group_name) REFERENCES alarmsapp.groups(group_name)
);

CREATE TRIGGER set_updated_at_trigger
BEFORE INSERT OR UPDATE ON alarmsapp.user_layouts 
FOR EACH ROW EXECUTE PROCEDURE alarmsapp.set_updated_at();

-- Generate the table that holds the timer information (snooze, bypass reminder, etc.). --
CREATE TABLE alarmsapp.timers (
	device VARCHAR(250),
	end_time TIMESTAMP NOT NULL,
	timer_type TEXT NOT NULL,
	updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
	updated_by CITEXT NOT NULL,
	PRIMARY KEY (device, timer_type)
);

CREATE TRIGGER set_updated_at_trigger
BEFORE INSERT OR UPDATE ON alarmsapp.timers
FOR EACH ROW EXECUTE PROCEDURE alarmsapp.set_updated_at();
