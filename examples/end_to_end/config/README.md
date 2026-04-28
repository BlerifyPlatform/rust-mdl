# config/

Drop your project's secrets here. Everything except this README and the
`.gitignore` is git-ignored.

```
config/
├── credentials.json    # service-account JSON from the Blerify portal
├── signing-key.pem     # PKCS#8 EC P-256 private key registered for the project
└── issuer-cert.pem     # PEM X.509 cert wrapping the matching public key
```

The example expects these exact filenames by default; override via
`BLERIFY_CREDS_PATH` / `BLERIFY_SIGNING_KEY_PATH` / `BLERIFY_ISSUER_CERT_PATH`
if you keep them elsewhere.
