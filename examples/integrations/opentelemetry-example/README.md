1. start postgres & openobserve
   ```sh
   docker compose up -d
   ```
2. open http://localhost:5080 to access openobserve, copy access token to `OTEL_EXPORTER_OTLP_HEADERS` in `.env`
   ```sh
    OTEL_EXPORTER_OTLP_HEADERS="authorization=Basic <token>,organization=default"
   ```
3. run example
   ```sh
   cargo run
   ```
4. view openobserve log & tracing