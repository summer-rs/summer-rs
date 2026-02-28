# Changelog

## 0.5.0

- **changed**: Refactor component ([#216])
- **added**: add component macro for declarative component ([#214])
- **breaking**: migrate `summer-job` & `summer-stream` extractor to `summer` crate ([#214], [#216])

[#216]: https://github.com/summer-rs/summer-rs/pull/216
[#214]: https://github.com/summer-rs/summer-rs/pull/214

**Migrating from 0.4 to 0.5**

```diff
-use summer_job::extractor::Component;
+use summer::extractor::Component;
-use summer_job::extractor::Config;
+use summer::extractor::Config;
 
 #[cron("1/10 * * * * *")]
 async fn cron_job(Component(db): Component<ConnectPool>) {
     let time: String = sqlx::query("select TO_CHAR(now(),'YYYY-MM-DD HH24:MI:SS') as time")
         .fetch_one(&db)
         .await
         .context("query failed")
         .unwrap()
         .get("time");
     println!("cron scheduled: {:?}", time)
 }
```


```diff
-use summer_stream::extractor::Component;
+use summer::extractor::Component;
-use summer_stream::extractor::Config;
+use summer::extractor::Config;

 #[stream_listener(
     "topic",
     kafka_consumer_options = fill_kafka_consumer_options
 )]
 async fn listen_topic_do_something(Component(db): Component<ConnectPool>, topic: StreamKey, Json(payload): Json<Payload>) {
     tracing::info!("received msg from topic#{}: {:#?}", topic, payload);
 }
```

## 0.4.7

- **changed**: upgrade `schemars` 0.9 to 1.1 ([#197])

[#197]: https://github.com/summer-rs/summer-rs/pull/197

## 0.4.6

- **added**: refactor `shutdown_signal` ([#180])

[#180]: https://github.com/summer-rs/summer-rs/pull/180

## 0.4.5

- **added**: Nested and circular dependency injection ([#173])

[#173]: https://github.com/summer-rs/summer-rs/pull/173

## 0.4.4

- **added**: export `summer::submit_config_schema`

## 0.4.3

- **added**: Service support inject None ([#160])

[#160]: https://github.com/summer-rs/summer-rs/pull/160

## 0.4.2

- **changed**: upgrade `toml` 0.8 to 0.9 ([#154])
- **changed**: upgrade `tokio` 1.44 to 1.46 ([#154])
- **changed**: upgrade `serde-toml-merge` 0.3.8 to 0.3.10 ([#154])

[#154]: https://github.com/summer-rs/summer-rs/pull/154

## 0.4.1

- **changed**: fix Immediately built plugins can't access config ([#145])

[#145]: https://github.com/summer-rs/summer-rs/pull/145

## 0.4.0

- **breaking**: upgrade `summer-macros` 0.3 to 0.4 ([#132])

[#132]: https://github.com/summer-rs/summer-rs/pull/132

**Migrating from 0.3 to 0.4**

```diff
 #[derive(Clone, Service)]
+#[service(prototype = "build")]
-#[prototype = "build"]
 struct UserService {
     #[inject(component)]
     db: ConnectPool,
     #[inject(config)]
     config: UserConfig,
 }
```

## 0.3.1

- **breaking**: remove `ComponentRegistry::create_service` ([#112])
- **added**: Added prototype service derived macro generation `build` function ([#112])

[#112]: https://github.com/summer-rs/summer-rs/pull/112

## 0.3.0

- **breaking**: refactor dependency inject ([#105])
- **changed**: use `TypeId` instead of `type_name` to improve performance ([#105])

[#105]: https://github.com/summer-rs/summer-rs/pull/105

**Migrating from 0.2 to 0.3**

```diff
 #[derive(Clone, Service)]
 struct UserService {
-    #[component]
+    #[inject(component)]
     db: ConnectPool,
-    #[config]
+    #[inject(config)]
     config: UserConfig,
 }
```

## 0.2.9

- **added**: toml support Environment variable interpolator ([#95])

[#95]: https://github.com/summer-rs/summer-rs/pull/95

## 0.2.8

- **added**: banner & Fancy Starting Logs ([#91])
- **added**: add `App::global()` ([#d1fa98])

[#91]: https://github.com/summer-rs/summer-rs/pull/91
[#d1fa98]: https://github.com/summer-rs/summer-rs/commit/d1fa983bc41750777c4bb12c5fa03479d273e977

## 0.2.7

- **added**: support `include_str!` compile configuration files into the application ([#85])
- **changed**: rename `config_file` to `use_config_file` ([#85])

[#85]: https://github.com/summer-rs/summer-rs/pull/85

## 0.2.6

- **changed**: fix concurrent scheduler ([#81])

[#81]: https://github.com/summer-rs/summer-rs/pull/81

## 0.2.5

- **changed**: fix exported `build` method ([#75])

[#75]: https://github.com/summer-rs/summer-rs/pull/75

## 0.2.4

- **added**: App add `get_env` ([#74])
- **added**: AppBuilder export `build` method ([#74])

[#74]: https://github.com/summer-rs/summer-rs/pull/74

## 0.2.3

- **added**: AppBuilder add `get_env` ([#65])
- **added**: AppBuilder add `add_layer` to support extends `tracing-rs` ([#65])
- **added**: support immediately Plugin ([#65])

[#65]: https://github.com/summer-rs/summer-rs/pull/65

## 0.2.2

- **added**: support shutdown hook ([#61])

[#61]: https://github.com/summer-rs/summer-rs/pull/61

## 0.2.1

- **added**: support config logger time pattern ([#59])
- **added**: support config logger add with_fields ([#59])

[#59]: https://github.com/summer-rs/summer-rs/pull/59

## 0.2.0

- **added**: add `Service` support dependency inject ([#54])
- **added**: add `get_component` ([#54])
- **breaking**: rename origin `get_component` to `get_component_ref` ([#54])

[#54]: https://github.com/summer-rs/summer-rs/pull/54

## 0.1.3

- **changed**: refactor toml config registry ([#1a750a])

[#1a750a]: https://github.com/summer-rs/summer-rs/commit/1a750a7d82871632bad7cee73ec418b5a28924ea

## 0.1.2

- **changed**: add DeserializeErr ([#44])

[#44]: https://github.com/summer-rs/summer-rs/pull/44

## before 0.1.1

see [CHANGELOG](../CHANGELOG.md)