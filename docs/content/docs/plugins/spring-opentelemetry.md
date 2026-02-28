+++
title = "opentelemetry Plugin"
description = "How to use the opentelemetry plugin"
draft = false
weight = 22
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "OpenTelemetry is a full-dimensional observability solution that integrates logs, metrics, and tracing."
toc = true
top = false
+++

## A brief history of OpenTelemetry

* In 2010, The [Dapper](https://research.google/pubs/dapper-a-large-scale-distributed-systems-tracing-infrastructure/) paper released by Google marked the beginning of distributed link tracing.
* In 2012, Twitter open-sourced [Zipkin](https://zipkin.io/).
* In 2015, Uber released the open-source version of [Jaeger](https://www.jaegertracing.io/). Currently, Zipkin and Jaeger are still one of the most popular distributed link tracing tools.
* In 2015, the [OpenTracing](https://opentracing.io/) project was accepted by [CNCF](https://www.cncf.io/) as its third hosted project, dedicated to standardizing distributed link tracing across components.
* In 2017, Google open-sourced its internal Census project, and then [OpenCensus](https://opencensus.io/) became popular in the community.
* In 2017, W3C started to develop [TraceContext](https://github.com/w3c/trace-context) related standards.
* In early 2019, two existing open source projects: OpenTracing and OpenCensus were announced to be merged into the [OpenTelemetry](https://opentelemetry.io/) project, and Log and Metrics were merged.
* In 2021, OpenTelemetry released V1.0.0, which provided stability guarantees for the client's link tracing part.
* 2023 is a milestone for OpenTelemetry, because its three basic signals, link tracing, metrics and logs, have all reached [stable versions](https://opentelemetry.io/status/).
* In 2024, [OpenTelemetry v1.3.0](https://github.com/open-telemetry/opentelemetry-proto/releases/tag/v1.3.0) was released, adding [profiling signals](https://opentelemetry.io/blog/2024/state-profiling/). In June of the same year, Elastic donated an EBPf-based [profiler agent](https://github.com/open-telemetry/opentelemetry-ebpf-profiler).

> [CNCF](https://landscape.cncf.io/?group=projects-and-products&view-mode=card&license=oss&classify=category&sort-by=stars&sort-direction=desc#observability-and-analysis--observability) has many observability backend implementations available for use.

{{ include(path="../../summer-opentelemetry/README.md") }}