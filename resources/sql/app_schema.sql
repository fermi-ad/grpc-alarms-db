/*
 * Script to generate the tables for the grpc-alarms-db service
 */

-- Create the trigger function for automatically setting the updated_at field in the tables. --
CREATE OR REPLACE FUNCTION alarmsapp.set_updated_at()
RETURNS TRIGGER AS 
$$
BEGIN
    NEW.updated_at := CURRENT_TIMESTAMP;
	RETURN NEW;
END;
$$
LANGUAGE plpgsql;

-- Generate the "parent" table that knows all the groups. --
CREATE TABLE alarmsapp.groups ( 
    group_name TEXT PRIMARY KEY,
	description TEXT,
	updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	updated_by TEXT NOT NULL 
);

CREATE TRIGGER set_updated_at_trigger
BEFORE INSERT OR UPDATE ON alarmsapp.groups 
FOR EACH ROW EXECUTE PROCEDURE alarmsapp.set_updated_at();

-- Generate the table that knows which groups and devices are inside a group. --
CREATE TABLE alarmsapp.group_membership (
    group_name TEXT,
	member_name TEXT,
	member_is_group BOOLEAN NOT NULL,
	updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	updated_by TEXT NOT NULL,
	PRIMARY KEY (group_name, member_name),
	FOREIGN KEY (group_name) REFERENCES alarmsapp.groups(group_name)
);

CREATE TRIGGER set_updated_at_trigger
BEFORE INSERT OR UPDATE ON alarmsapp.group_membership 
FOR EACH ROW EXECUTE PROCEDURE alarmsapp.set_updated_at();


-- Generate the table that knows all the top-level groups for a user (displayed on the alarm screen as categories). --
CREATE TABLE alarmsapp.user_layouts (
    user_name TEXT,
	group_name TEXT,
	updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	updated_by TEXT NOT NULL,
	PRIMARY KEY (user_name, group_name),
	FOREIGN KEY (group_name) REFERENCES alarmsapp.groups(group_name)
);

CREATE TRIGGER set_updated_at_trigger
BEFORE INSERT OR UPDATE ON alarmsapp.user_layouts 
FOR EACH ROW EXECUTE PROCEDURE alarmsapp.set_updated_at();

-- Generate the lookup table for the types of timers (snooze, bypass reminder, etc.). --
CREATE TABLE alarmsapp.timer_types (
	type_id INT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	type_name TEXT NOT NULL
);

-- Add timer types (ensure completeness and correctness if running this for real). --
INSERT INTO alarmsapp.timer_types (type_name)
VALUES 
    ('TimerType_SNOOZE'),
	('TimerType_BYPASS_REMINDER');

-- Generate the table that holds the timer information. --
CREATE TABLE alarmsapp.timers (
	device TEXT,
	end_time TIMESTAMP NOT NULL,
	timer_type INT NOT NULL,
	updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	updated_by TEXT NOT NULL,
	PRIMARY KEY (device, timer_type),
	FOREIGN KEY (timer_type) REFERENCES alarmsapp.timer_types(type_id)
);

CREATE TRIGGER set_updated_at_trigger
BEFORE INSERT OR UPDATE ON alarmsapp.timers
FOR EACH ROW EXECUTE PROCEDURE alarmsapp.set_updated_at();
