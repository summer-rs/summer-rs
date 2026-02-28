+++
title = "summer-mail插件"
description = "mail插件如何使用"
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "summer-mail是针对<a href='https://github.com/lettre/lettre' target='_blank'>lettre</a>的自动装配"
toc = true
top = false
+++


![lettre Repo stars](https://img.shields.io/github/stars/lettre/lettre) ![downloads](https://img.shields.io/crates/d/lettre.svg)
lettre是Rust最流行的邮件客户端，并且支持异步API。summer-mail主要使用它的tokio异步API。


{{ include(path="../../summer-mail/README.zh.md") }}