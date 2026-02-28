+++
title = "Data comparison between rust's summer-rs and java's SpringBoot"
description = "The Benchmark of the summer-rs"
sort_by = "weight"
date = 2024-09-04T09:19:42+00:00
updated = 2024-09-04T09:19:42+00:00
weight = 3
draft = false
template = "blog/page.html"
+++

The size of the release binary built with summer-rs is half of the SpringBoot jar package. [Rust still have a lot of room for optimization](https://github.com/johnthagen/min-sized-rust).
<img width="100%" alt="Build Target Size" src="https://quickchart.io/chart?c={type:%27bar%27,data:{labels:[%22java%27ssummer-boot%22,%22rust%27ssummer-rs%22],datasets:[{label:%22build%20target%20size(MB)%22,data:[22.25,11.17]}]}}&format=svg"/>

The size of the Docker image built with summer-rs is 1/4 of the SpringBoot image. [The rust docker image also has a lot of room for optimization](https://github.com/kpcyrd/mini-docker-rust).
<img width="100%" alt="Docker Image Size" src="https://quickchart.io/chart?c={type:%27bar%27,data:{labels:[%22java-SpringBoot%22,%22rust-summer-rs%22],datasets:[{label:%27docker%20image%20size(MB)%27,data:[429.99,124.55]}]}}&format=svg"/>

The runtime memory usage of using summer-rs is 1/10 of that of SpringBoot.
<img width="100%" alt="Runtime Memory Usage" src="https://quickchart.io/chart?c={type:%27bar%27,data:{labels:[%22java-SpringBoot%22,%22rust-summer-rs%22],datasets:[{label:%27Runtime%20Memory%20Usage(MB)%27,data:[234.6,21.2]}]}}&format=svg"/>

The QPS of the simplest web application using summer-rs is twice that of SpringBoot.
<img width="100%" alt="Raw Query QPS" src="https://quickchart.io/chart?c={type:%27bar%27,data:{labels:[%22java-SpringBoot%22,%22rust-summer-rs%22],datasets:[{label:%27Raw%20Query%20QPS%27,data:[24805.60,40143.45]}]}}&format=svg"/>

The QPS of a web application with database queries using summer-rs is basically the same as SpringBoot.
<img width="100%" alt="Postgres Query QPS" src="https://quickchart.io/chart?c={type:%27bar%27,data:{labels:[%22java-SpringBoot%22,%22rust-summer-rs%22],datasets:[{label:%27Postgres%20Query%20QPS%27,data:[9679.59,9250.40]}]}}&format=svg"/>

The data query tool currently used is `sqlx`. [sqlx's performance support for MySQL](https://github.com/launchbadge/sqlx/issues/1481) is very poor, and the stress test results are only half of SpringBoot, so it is recommended to use PostgreSQL as the backend of sqlx.

Next, I will connect to [rust-postgres](https://github.com/sfackler/rust-postgres) to see if the performance will be improved compared to sqlx.

Detailed stress test code and related data can be found in [this link](https://github.com/summer-rs/summer-benchmark)