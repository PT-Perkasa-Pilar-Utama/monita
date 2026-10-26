-- Monita no longer limits callers per address (v1.1 Q45): a shared office or carrier address
-- would lock out everyone behind it. Sign-in is limited per account instead, in code.
DELETE FROM settings WHERE key = 'rl_ip_per_min';
