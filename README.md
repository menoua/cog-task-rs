# CogTask

[![Crates.io Version](https://img.shields.io/crates/v/cog-task.svg)](https://crates.io/crates/cog-task)
[![Crates.io Downloads](https://img.shields.io/crates/d/cog-task.svg)](https://crates.io/crates/cog-task)
[![Build Status](https://github.com/menoua/cog-task-rs/workflows/CI/badge.svg)](https://github.com/menoua/cog-task-rs/actions)
[![License](https://img.shields.io/crates/l/cog-task.svg)](https://opensource.org/licenses/MIT)

A general-purpose low-latency application to serve cognitive tasks, built with [egui](https://github.com/emilk/egui).

## Installation

The most reliable way to install CogTask is by installing Cargo through [rustup](https://rustup.rs/) and compiling the binaries locally.

1. Install Cargo:<br>
```$ curl https://sh.rustup.rs -sSf | sh```

2. Build binaries (choose one):
   - Build stable binaries from [crates.io](https://crates.io/crates/cog-task):<br>
   ```$ cargo install cog-task [--features=...]```
   
   - Build nightly binaries from [github](https://github.com/menoua/cog-task-rs):<br>
   ```$ cargo install --git https://github.com/menoua/cog-task-rs [--features=...]```

## Features

By default (no features), this package should compile and run out-of-the-box on a reasonably recent macOS or Linux distribution. Some types of actions however depend on features that can be enabled during installation. These features are not enabled by default because they rely on system libraries that might not be installed on the OS out-of-the-box.

Currently, there are 4 main features that can be enabled:
1. `audio` -- enables the `Audio` action via the ALSA sound library.
2. `gstreamer` -- enables the `Stream` and `Video` actions via the gstreamer backend.
3. `ffmpeg` -- enables the `Stream` and `Video` actions via the ffmpeg backend.
4. `full` -- a shorthand to enable the previous three features.

For example:
- Stable binaries with full support:<br>
```$ cargo install cog-task --features=full```
- Nightly binaries with audio and gstreamer support:<br>
```$ cargo install --git https://github.com/menoua/cog-task-rs --features=audio,gstreamer```

## Usage

This crate installs two binaries -- `cog-launcher` and `cog-server`.

`cog-launcher`: A launcher that provides a graphical interface to find and load tasks from disk.

`cog-server /path/to/task`: Used to run a specific task by providing the path to its directory. `cog-launcher` runs this binary when starting a task, so make sure both binaries are in the same directory.

## Changelog

Version 0.2.0 has gone through a massive overhaul, transitioning from the GUI framework of `iced` to `egui`. The transition was done to solve a screen update skipping issue (which it did). There have been other pros and cons too. Text and widget styling is (much) more difficult in `egui`. `egui`'s Glow backend supports image/SVG. Separating the `view` and `update` calls allowed redesigning block architecture (the dependency graph) into an action tree. This change makes it very difficult to design a buggy task, and significantly simplifies task definition style. It slightly limits the task design flexibility, but it's worth it. This change also comes with an increased overhead since `update`/`view` calls traverse the entire active subset of the tree, instead of jumping to the end nodes. However, the tree overhead is generally low compared to action-specific overheads, so that's not a huge deal.
