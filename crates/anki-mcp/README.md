# anki-mcp

Read-only stdio MCP adapter for `anki-tool`. It uses the official Rust MCP SDK
and exposes `cards`, `decks`, `due`, `find_cards`, `forecast`, `hard`, `history`,
`new_cards`, `overview`, `progress`, `reviews`, `session`, `stats`, and `subdecks`.
There is no shell tool, arbitrary AnkiConnect action, sync tool, or card mutation.

```sh
nix build .#anki-mcp .#anki-tool
anki-mcp --anki-tool /absolute/path/to/anki-tool
```

Without `--anki-tool`, the adapter resolves `anki-tool` from `PATH`. Its stdout
is reserved for MCP messages. Tool calls return the CLI's JSON as text, with
Anki failures reported as MCP tool errors. Calls are serialized; simultaneous
calls receive a retryable busy error. Each subprocess has a 90-second deadline,
a 1 MiB stdout limit, and a 16 KiB stderr limit. Queries, card lists, lookbacks,
and card counts are validated before spawning a process. Subprocesses are killed
on timeout or cancellation. Card text is untrusted data.

## Managed host

`lab.hosts.<host>.anki-mcp.enable = true` installs the adapter, `anki-tool`,
Anki with declarative AnkiConnect settings, and the pinned OpenAI tunnel client
on Linux. Lynx enables this feature.

- `anki-headless.service` runs Anki under Xvfb as the host's normal user, using
  a dedicated base directory at `/var/lib/anki-headless`. It checks that port
  8765 is free, waits for AnkiConnect readiness, and restarts on failure.
- `anki-headless-sync.timer` syncs with AnkiWeb every five minutes. Set
  `anki-mcp.syncInterval = null` to disable it. Sync failures requiring a GUI
  decision must be resolved interactively.
- `anki-mcp-tunnel.service` runs the tunnel client as a separate dynamic user,
  which launches the MCP adapter and can reach AnkiConnect on loopback. It cannot
  read the Anki profile or the host user's home directory.

AnkiConnect listens only on `127.0.0.1:8765`, with browser origins disabled. No
firewall ports are opened. The tunnel health/admin listener is restricted to
`127.0.0.1:8766`. The tunnel API key is loaded through systemd `LoadCredential`,
then passed to the client as a file reference, never as a literal argument or
environment variable. The profile, AnkiWeb credentials, tunnel identity, and
runtime API key remain outside the repository and Nix store.

## Provision lynx

These steps happen after deploying the configuration; building the flake does
not create a tunnel, link a ChatGPT account, or sign in to AnkiWeb. Services skip
startup until their provisioning files exist.

1. In lynx's desktop session, close any running Anki instance. Only one process
   on the host can listen on port 8765. Create the service's private directory
   as the logged-in host user:

   ```sh
   sudo install -d -m 0700 -o "$USER" /var/lib/anki-headless
   anki -b /var/lib/anki-headless -p 'User 1'
   ```

   Complete initial setup, sign in to AnkiWeb, and download the existing
   collection. Verify the selected profile and collection before any full-sync
   decision. Disable automatic update dialogs in Anki's preferences. Perform a
   successful sync and verify `anki-tool overview` against the expected data.
   Close Anki cleanly, then mark the profile ready:

   ```sh
   touch /var/lib/anki-headless/.provisioned
   sudo systemctl start anki-headless.service
   ```

   If the profile has another name, set `anki-mcp.profile` to match it. Use this
   managed profile as lynx's Anki instance. To use its GUI, stop the services,
   open it with the same `-b` and `-p` arguments, close it, and restart services.
   Never copy a live collection database between profiles.

2. Create a private tunnel in [Platform tunnel settings](https://platform.openai.com/settings/organization/tunnels).
   Associate only the intended personal Platform organization and ChatGPT
   workspace. Create a restricted runtime key with Tunnels Read + Use; do not
   use an organization admin key for the daemon.

   Create root-only runtime files and edit them locally, without putting the
   key in command arguments, shell history, or chat:

   ```sh
   sudo install -d -m 0700 /etc/anki-mcp
   sudo install -m 0600 /dev/null /etc/anki-mcp/api-key
   sudo install -m 0600 /dev/null /etc/anki-mcp/tunnel.env
   sudoedit /etc/anki-mcp/api-key /etc/anki-mcp/tunnel.env
   ```

   `api-key` contains only the runtime key. `tunnel.env` contains:

   ```text
   CONTROL_PLANE_TUNNEL_ID=<the complete tunnel ID from Platform>
   ```

   Optional `CONTROL_PLANE_ORGANIZATION_ID` belongs in the same environment file.
   Override `anki-mcp.tunnelApiKeyFile` and `anki-mcp.tunnelEnvironmentFile` when
   using a different private provisioning mechanism. Do not run the creation
   commands again when rotating credentials; edit the existing files instead.

3. Start the tunnel and verify readiness:

   ```sh
   sudo systemctl start anki-mcp-tunnel.service
   systemctl status anki-headless.service anki-mcp-tunnel.service
   curl --fail http://127.0.0.1:8766/readyz
   anki-tool overview
   ```

   Use `journalctl -u anki-headless -u anki-mcp-tunnel` for failures. Keep logs
   private; application errors may contain collection information.

4. In ChatGPT's custom MCP setup, choose **Tunnel**, select the same tunnel,
   create and install a private plugin, and request an overview. Compare it
   with the direct CLI output and test a search and struggling-card query.

   This deployment relies on the tunnel's organization/workspace access boundary
   and a private plugin in a single-user context. It does not implement end-user
   OAuth. Do not share the plugin or associate a shared workspace with the tunnel;
   add OAuth and account-level authorization before allowing other users access.
   An IP allowlist would identify OpenAI traffic rather than your individual
   account and cannot provide that authorization.

5. Verify recovery after closing Anki unexpectedly, restarting the tunnel,
   logging out, and rebooting lynx. Verify an unassociated account/workspace
   cannot discover or use the tunnel. The services are system services and do
   not depend on a desktop login. ChatGPT data reflects the last completed
   AnkiWeb sync, which can lag changes on other devices.

For interactive maintenance:

```sh
sudo systemctl stop anki-headless-sync.timer anki-headless-sync.service
sudo systemctl stop anki-headless.service
anki -b /var/lib/anki-headless -p 'User 1'
# Close Anki before restarting the managed instance.
sudo systemctl start anki-headless.service anki-mcp-tunnel.service anki-headless-sync.timer
```

The tunnel stops with Anki through its systemd dependency. Restart it explicitly
after maintenance or provisioning. If sync needs a full-download/upload choice,
resolve it in this interactive session before returning to unattended operation.

## Development

```sh
nix develop . --command cargo test --manifest-path crates/Cargo.toml -p anki-mcp
nix develop . --command cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets -- -D warnings
nix fmt
```

Tests exercise the MCP handshake/discovery/error path, literal argument
handling, rejected commands and bounds, subprocess failures, deadlines, and
response-size limits without accessing a real collection or OpenAI account.

Protocol and deployment references:

- [OpenAI Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels)
- [Custom MCP server setup](https://developers.openai.com/api/docs/guides/custom-mcp-server)
- [Tunnel client v0.0.19 configuration](https://github.com/openai/tunnel-client/blob/v0.0.19/docs/configuration.md)
- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
