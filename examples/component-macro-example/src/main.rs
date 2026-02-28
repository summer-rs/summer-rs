use serde::Deserialize;
use summer::component;
use summer::config::Configurable;
use summer::plugin::ComponentRegistry;
use summer::App;

// Define configuration
#[derive(Debug, Clone, Configurable, Deserialize)]
#[config_prefix = "database"]
struct DbConfig {
    host: String,
    port: u16,
}

// Define components
#[derive(Clone, Debug)]
struct DbConnection {
    #[allow(dead_code)]
    url: String,
}

#[derive(Clone, Debug)]
struct UserRepository {
    #[allow(dead_code)]
    db: DbConnection,
}

#[derive(Clone, Debug)]
struct UserService {
    #[allow(dead_code)]
    repo: UserRepository,
}

// Use #[component] macro to register components
#[component]
fn create_db_connection(
    summer::extractor::Config(config): summer::extractor::Config<DbConfig>,
) -> DbConnection {
    println!("Creating DbConnection with config: {:?}", config);
    DbConnection {
        url: format!("{}:{}", config.host, config.port),
    }
}

#[component]
fn create_user_repository(
    summer::extractor::Component(db): summer::extractor::Component<DbConnection>,
) -> UserRepository {
    println!("Creating UserRepository with db: {:?}", db);
    UserRepository { db }
}

#[component]
fn create_user_service(
    summer::extractor::Component(repo): summer::extractor::Component<UserRepository>,
) -> UserService {
    println!("Creating UserService with repo: {:?}", repo);
    UserService { repo }
}

#[tokio::main]
async fn main() {
    println!("Starting component-macro-example...");

    let app = App::new().build().await.expect("Failed to build app");

    // Get components
    let db = app
        .get_component::<DbConnection>()
        .expect("DbConnection not found");
    println!("Got DbConnection: {:?}", db);

    let repo = app
        .get_component::<UserRepository>()
        .expect("UserRepository not found");
    println!("Got UserRepository: {:?}", repo);

    let service = app
        .get_component::<UserService>()
        .expect("UserService not found");
    println!("Got UserService: {:?}", service);

    println!("All components registered successfully!");
}
