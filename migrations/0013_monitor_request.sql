-- A monitor can send a method other than GET, headers (such as Authorization) and a body. The
-- header values and the body are stored as entered: the check must send them.
ALTER TABLE monitors ADD COLUMN method TEXT NOT NULL DEFAULT 'GET' CHECK (method IN ('GET', 'HEAD', 'POST', 'PUT'));
ALTER TABLE monitors ADD COLUMN headers TEXT;
ALTER TABLE monitors ADD COLUMN body TEXT;
