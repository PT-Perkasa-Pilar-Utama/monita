-- AC-15.02: an unchanged alert setup emails someone. Projects made before this migration got
-- empty recipient lists, so their alerts went nowhere. A route that sends nothing (no email and
-- no webhook in use) now emails the project owner. A webhook-only route stays as it is.
UPDATE alert_routes
SET emails = (
  SELECT u.email FROM projects p JOIN users u ON u.id = p.owner_id WHERE p.id = alert_routes.project_id
)
WHERE emails = '' AND (use_webhook = 0 OR webhook_url IS NULL OR webhook_url = '');
