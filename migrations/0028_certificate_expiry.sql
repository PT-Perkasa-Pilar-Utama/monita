-- An HTTPS monitor may warn before its certificate expires (v1.1 decision Q25). Off when NULL.
ALTER TABLE monitors ADD COLUMN cert_warn_days INTEGER CHECK (cert_warn_days IN (7, 14, 30));
-- The notAfter of the leaf certificate the last HTTPS check saw, in Unix seconds.
ALTER TABLE monitors ADD COLUMN cert_expires_at INTEGER;
-- The notAfter the last warning was about, so each certificate warns once.
ALTER TABLE monitors ADD COLUMN cert_warned_for INTEGER;
