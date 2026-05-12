use std::sync::Arc;

use summer_web::rmpv::Value;
use summer_web::socketioxide::SocketIo;
use summer_web::socketioxide::extract::{AckSender, Data, Event, SocketRef};
use summer::plugin::MutableComponentRegistry;
use summer::tracing::info;
use summer::{auto_config, App};
use summer_web::{on_fallback, WebConfigurator};
use summer_web::extractor::Component;
use summer_web::{
    axum::{
        response::IntoResponse,
    },
    WebPlugin,
};
use summer_web::{get, on_connection, on_disconnect, subscribe_message};
use tokio::sync::Mutex;

type Users = Arc<Mutex<Vec<String>>>;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    let users = Vec::<String>::new();

    App::new()
        .add_plugin(WebPlugin)
        .add_component(Arc::new(Mutex::new(users)))
        .run()
        .await
}

#[get("/users-online")]
async fn show_users_online(Component(users): Component<Users>) -> impl IntoResponse {
    let users = users.lock().await;
    format!("{users:?}")
}

#[get("/emitter")]
async fn emitter(Component(io): Component<SocketIo>) -> impl IntoResponse {
    io.emit("message", &"hello from http").await.ok();
    "emitted"
}

#[on_connection]
async fn on_connection(socket: SocketRef, Component(users): Component<Users>) {
    info!(ns = socket.ns(), ?socket.id, "Socket.IO connected");
    let mut users_lock = users.lock().await;
    users_lock.push(socket.id.to_string());
}

#[subscribe_message("message")]
async fn message(socket: SocketRef, Data(data): Data<Value>){
    info!(?socket.id, ?data, "Received event:");
    socket.emit("message-back", "hello").ok();
}

#[subscribe_message("message-with-ack")]
async fn message_with_ack(Event(_event): Event, Data(data): Data<Value>, ack: AckSender, Component(_users): Component<Users>) {
    info!(?data, "Received event with ack:");
    ack.send("ack").ok();
}

#[on_disconnect]
async fn on_disconnect(socket: SocketRef, Component(users): Component<Users>) {
    info!(ns = socket.ns(), ?socket.id, "Socket.IO disconnected");
    let mut users_lock = users.lock().await;
    users_lock.retain(|u| u != &socket.id.to_string());
}

#[on_fallback]
/// The handlers are flexible, you can choose to accept `Event`, `Data` or any other
/// extractors as parameters, and also you can choose to accept `Component` parameters
async fn on_fallback(socket: SocketRef, Event(_event): Event, Data(data): Data<Value>, Component(_users): Component<Users>) {
    info!(?socket.id, ?data, "Received fallback event:");
    socket.emit("fallback", "This event is not handled").ok();
}