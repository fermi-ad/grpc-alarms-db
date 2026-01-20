/*
 * Script to populate the grpc-alarms-db tables from legacy data
 */

-- Pull in basic alarm lists --
INSERT INTO alarmsapp.groups (group_name, description, updated_by)
SELECT name, long_name, modify_user_name
FROM hendricks.alarm_list_info;

-- Create groups for each of the user categories --
INSERT INTO alarmsapp.groups (group_name, description, updated_by)
SELECT DISTINCT username || '_' || title AS group_name, title || ' - User category', username
FROM ahn.alarmdspwinpar;

-- Pull in user layouts from alarmdspwinpar table --
INSERT INTO alarmsapp.user_layouts (group_name, user_name, updated_by)
SELECT DISTINCT username || '_' || title AS group_name, username, username 
FROM ahn.alarmdspwinpar;

-- Put devices in the group_membership table --
INSERT INTO alarmsapp.group_membership (group_name, member_name, member_is_group, updated_by)
SELECT DISTINCT l.name, d.name, false AS member_is_group, 'HENDRICKS' AS updated_by 
FROM accdb.alarm_block b 
    INNER JOIN accdb.device d 
	    ON d.di = b.di 
	INNER JOIN hendricks.alarm_list_nodes n 
	    ON d.trunk = n.trunk 
		AND d.node = n.node 
    INNER JOIN hendricks.alarm_list_info l 
	    ON 
		    CASE 
			    WHEN d.alarm_list_id > 0 
				    THEN d.alarm_list_id = l.list_number
				ELSE n.list_number = l.list_number 
			END
;

-- Prepare to map the basic alarm lists into the user layout groups --
CREATE TABLE alarmsapp.tempcutover (
    username TEXT,
	list_index INT,
	layout_group_mapping CHAR(1),
	list_name TEXT,
	user_list_index INT,
	user_list_name TEXT
);

-- Break apart the mapdat field from alarmdspmap --
INSERT INTO alarmsapp.tempcutover (username, list_index, layout_group_mapping)
SELECT username, i, substring(m.mapdat, i, 1)
FROM ahn.alarmdspmap m, generate_series(1, length(m.mapdat)) AS i;

-- Map indices to list names --
UPDATE alarmsapp.tempcutover t 
SET list_name = (
    SELECT name 
	FROM hendricks.alarm_list_info h 
	WHERE t.list_index = h.list_number
);

-- Remove unmapped rows --
DELETE FROM alarmsapp.tempcutover 
WHERE layout_group_mapping = '0'
    OR list_name IS NULL;

-- Find user category for each row by converting hex values to base 10 --
UPDATE alarmsapp.tempcutover 
SET user_list_index = ('0x' || layout_group_mapping)::numeric;

-- Map user category names to each row --
UPDATE alarmsapp.tempcutover t 
SET user_list_name = (
    SELECT title 
	FROM ahn.alarmdspwinpar a 
	WHERE a.username = t.username 
	    AND a.indx = t.user_list_index 
);

-- Remove rows without a user category --
DELETE FROM alarmsapp.tempcutover 
WHERE user_list_name IS NULL;

-- Map the basic lists to the user categories --
INSERT INTO alarmsapp.group_membership (group_name, member_name, member_is_group, updated_by)
SELECT username || '_' || user_list_name, list_name, true, username 
FROM alarmsapp.tempcutover;

-- Clean up --
DROP TABLE alarmsapp.tempcutover;
