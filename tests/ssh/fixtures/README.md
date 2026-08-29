# SSH Fixture Policy

The unit suite uses an in-process `SshRunner` fake so it never needs a private
key, password, token, or a listening SSH port. `docker-compose.yml` is an
opt-in, read-only container placeholder for manual transport experiments; it
contains no credentials, session content, or exposed ports.

Do not add real SSH keys, `known_hosts`, user session files, prompts, commands,
or secrets here. End-to-end system OpenSSH verification belongs on a disposable
host using the operator's existing configuration.
