-- A severity's webhook starts unticked (v1.1 decision Q27). A route ticked with no URL could never
-- send anything, and its form showed a required field the owner never filled; it is unticked.
-- A route with a saved URL keeps what its owner chose.
UPDATE alert_routes SET use_webhook = 0 WHERE webhook_url IS NULL OR trim(webhook_url) = '';
