-- The address where people use the project's system. New projects must give one; the status
-- page links to it. NULL for projects made before it was asked.
ALTER TABLE projects ADD COLUMN homepage_url TEXT;
