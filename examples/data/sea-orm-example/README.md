1. start postgres
    ```sh
    docker compose up -d
    ```
2. run web server
    ```sh
    cargo run
    ```
3. open browser
    ```
    http://localhost:8080/
    http://localhost:8080/1
    ```

**generate entities**
```sh
sea-orm-cli generate entity --with-serde both --output-dir src/entities
```