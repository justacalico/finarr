#!/bin/sh
# Fix ownership of the bind-mounted data dir (docker creates it root-owned),
# then drop to the app user.
if [ "$(id -u)" = "0" ]; then
  chown -R finarr:finarr /data 2>/dev/null || true
  exec su -s /bin/sh finarr -c "exec finarr $*"
fi
exec finarr "$@"
