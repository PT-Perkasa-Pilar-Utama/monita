-- A monitor checks HTTP, as before, a TCP port or a DNS name (v1.1 decision Q16). A TCP check
-- keeps `host:port` in url and a DNS check the host name. HTTP-only columns keep their defaults.
ALTER TABLE monitors ADD COLUMN kind TEXT NOT NULL DEFAULT 'http' CHECK (kind IN ('http', 'tcp', 'dns'));
-- A DNS check passes only when one A or AAAA answer equals this address, when it is set.
ALTER TABLE monitors ADD COLUMN expected_address TEXT;
