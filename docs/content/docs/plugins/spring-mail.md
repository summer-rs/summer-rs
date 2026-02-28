+++
title = "summer-mail Plugin"
description = "How to use the summer-mail plugin"
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "summer-mail is an automatic assembly for <a href='https://github.com/lettre/lettre' target='_blank'>lettre</a>"
toc = true
top = false
+++

![lettre Repo stars](https://img.shields.io/github/stars/lettre/lettre) ![downloads](https://img.shields.io/crates/d/lettre.svg)
Lettre is the most popular mail client in Rust and supports asynchronous API. summer-mail mainly uses its tokio asynchronous API.

{{ include(path="../../summer-mail/README.md") }}