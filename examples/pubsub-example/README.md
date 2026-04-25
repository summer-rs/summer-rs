# Pubsub Example 

For this example, we will use the Pubsub API to send and receive messages, so you need to have a Pubsub topic created and a service account with the necessary permissions.

Then save the credentials to the `credentials.json` file and execute the docker compose to start the postgres database.

```bash
docker compose up -d
```

Then execute the following command to start the application.

```bash
cargo run
```

Visiting the endpoint `http://localhost:8080/auth/signup` will send a message to the Pubsub topic `users_created` and the consumer will receive the message.

The `project_id` in the `config/app.toml` file must be the same as the project id of the credentials file.

