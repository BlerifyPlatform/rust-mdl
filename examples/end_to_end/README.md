# end_to_end example

Runnable demonstration of the full Issuance API flow against any
environment (default: `https://api.demo.blerify.com`):

```
generate → sign locally with EC P-256 (ES256) → assemble → revoke
```

## What you provide

The signing keypair must be the one **registered for your project** on
Blerify's side. A randomly generated keypair will fail server validation
even if everything else is correct.

Drop these three files into `config/` (gitignored — see `config/README.md`):

| File | Format | Source |
|---|---|---|
| `credentials.json` | Service-account JSON | Blerify portal → service accounts |
| `signing-key.pem` | EC P-256 private key (PKCS#8 or SEC1 PEM both accepted) | Your project's registered signing key |
| `issuer-cert.pem` | PEM X.509 cert | Wraps the public half of `signing-key.pem` |

## Required environment variables

| Variable | Purpose |
|---|---|
| `BLERIFY_PROJECT_ID` | Blerify project UUID |
| `BLERIFY_TEMPLATE_ID` | mDL template UUID inside that project |
| `BLERIFY_KID` | `kid` registered for the cert/key pair |

## Optional environment variables

| Variable | Default |
|---|---|
| `BLERIFY_BASE_URL` | `https://api.demo.blerify.com` |
| `BLERIFY_CREDS_PATH` | `examples/end_to_end/config/credentials.json` |
| `BLERIFY_SIGNING_KEY_PATH` | `examples/end_to_end/config/signing-key.pem` |
| `BLERIFY_ISSUER_CERT_PATH` | `examples/end_to_end/config/issuer-cert.pem` |
| `RUST_LOG` | `info` |

## Run

The example automatically loads `examples/end_to_end/.env` if present (set
your env vars there once instead of exporting them every run):

```bash
cp examples/end_to_end/.env.example examples/end_to_end/.env
$EDITOR examples/end_to_end/.env       # fill in PROJECT_ID, TEMPLATE_ID, KID
cargo run --example end_to_end
```

`.env` is git-ignored; `.env.example` is the committed template. Shell env
vars still take precedence over `.env` for one-off overrides.

You should see four steps print, ending with `✓ end-to-end flow completed`.
The example revokes its own test credential at the end so the run is
self-cleaning.

## Verbose logs

```bash
RUST_LOG=debug,rust_mdl=trace cargo run --example end_to_end
```

This surfaces the tracing spans on every API call (auth refresh, generate,
assemble, revoke) plus header / body trace.
