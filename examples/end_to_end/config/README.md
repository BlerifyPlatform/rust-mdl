# config/

```
config/
├── credentials.json    # YOU SUPPLY — service-account JSON from the Blerify portal
├── signing-key.pem     # YOU SUPPLY — PKCS#8 EC P-256 private key registered for the project
└── issuer-cert.pem     # SHIPPED   — PEM X.509 cert wrapping the matching public key
```

`credentials.json` and `signing-key.pem` are git-ignored — drop your own
in here.

`issuer-cert.pem` is checked in: it is the certificate associated with
the test keypair the demo project is registered against (the same cert
shipped in the `php-mdl` repository's `index.php`). Useful as-is when
the matching private key is available; replace with your own cert when
running against a different project.

The example expects these exact filenames by default; override via
`BLERIFY_CREDS_PATH` / `BLERIFY_SIGNING_KEY_PATH` / `BLERIFY_ISSUER_CERT_PATH`
if you keep them elsewhere.
