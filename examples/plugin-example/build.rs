pub fn main() {
    let _ = summer_apalis::ApalisPlugin;
    let _ = summer_grpc::GrpcPlugin;
    let _ = summer_job::JobPlugin;
    let _ = summer_mail::MailPlugin;
    let _ = summer_opendal::OpenDALPlugin;
    let _ = summer_opentelemetry::OpenTelemetryPlugin;
    let _ = summer_postgres::PgPlugin;
    let _ = summer_redis::RedisPlugin;
    let _ = summer_sea_orm::SeaOrmPlugin;
    let _ = summer_sqlx::SqlxPlugin;
    let _ = summer_sa_token::SaTokenPlugin;
    let _ = summer_stream::StreamPlugin;
    let _ = summer_web::WebPlugin;

    let r = summer::config::write_merged_schema_to_file("../../target/config-schema.json");
    println!("{r:?}")
}
