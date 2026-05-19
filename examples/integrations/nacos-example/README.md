# nacos-example

Demonstrates `summer-nacos` with [r-nacos](https://github.com/nacos-group/r-nacos).

```bash
cd examples/integrations/nacos-example
docker compose up -d
```

In the r-nacos console (`http://127.0.0.1:10848/rnacos/`, admin/admin), create config:

- **dataId**: `app.toml`
- **group**: `DEFAULT_GROUP`
- **content**: any TOML or text

```bash
cargo run
curl http://127.0.0.1:8080/
```

The instance `nacos-example` appears under service discovery after the web server starts.
