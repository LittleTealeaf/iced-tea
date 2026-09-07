# iced-tea

[![GitHub Tag](https://img.shields.io/github/v/tag/LittleTealeaf/iced-tea)](https://github.com/LittleTealeaf/iced-tea/tags)
[![Documentation](https://img.shields.io/badge/docs-github_pages-blue.svg)](https://littletealeaf.github.io/iced-tea/iced_tea/)
[![CI](https://github.com/LittleTealeaf/iced-tea/actions/workflows/rust.yml/badge.svg)](https://github.com/LittleTealeaf/iced-tea/actions/workflows/rust.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Elm Architecture (TEA) extensions, effect combinators, and component traits for [Iced](https://iced.rs/).

---

## Overview

When developing scalable graphical user interfaces with [Iced](https://iced.rs/), managing nested state transitions, parent-child message delegation, asynchronous task side-effects, and error recovery often leads to boilerplate and complex glue code.

`iced-tea` provides structured primitives and combinators to streamline Elm Architecture workflows in Iced:
- **`Signal<M, O>`**: A unified effect and control-flow type replacing raw commands. Handles internal messages, parent outbound events, async tasks, batching, sequencing, and graceful error boundaries.
- **`Model`**: Hierarchical component state machines with support for borrowed contexts and outbound message mapping (`map_update`, `empty_update`).
- **`Component`**: Self-contained UI building blocks combining `Model` state transitions with declarative view rendering (`render`, `render_into`, `view`, `view_into`).
- **`App`**: A cohesive runtime bridge connecting your root component directly into `iced::application`.

---

## Core Concepts

### `Signal<M, O>`
A `Signal` represents the result of a state transition:
- `Signal::Message(M)`: Dispatches an internal message back to the local component update loop.
- `Signal::Out(O)`: Emits an outbound event for parent components to intercept and handle.
- `Signal::Task(Task<M>)`: Asynchronous runtime task producing an internal message upon completion.
- `Signal::Batch(Vec<Signal>)`: Multiple signals executed concurrently or without strict ordering.
- `Signal::Sequence(Vec<Signal>)`: Signals executed in strict sequential order.
- `Signal::OnError(Box<Signal>, Option<M>)`: Fault-tolerance wrapper triggering a fallback message if an error occurs.
- `Signal::Done`: Successful completion with no further action.

### `Model`
Represents an isolated state machine with:
- `Message`: Local actions processed by the model.
- `OutMessage`: Outbound notifications emitted to parent models.
- `Context<'a>`: Temporary state borrowed during updates (e.g. database handles, themes).
- `update`: Handles a message, updates state, and returns `anyhow::Result<Signal<Message, OutMessage>>`.
- `map_update`: Automatically translates child `OutMessage` variants into parent signals.
- `empty_update`: Lifts child models with no outbound events (`OutMessage = ()`) into parent message domains.

### `Component`
Extends `Model` with view rendering capabilities:
- `render`: Renders the component into an `iced::Element` producing local messages.
- `render_into`: Renders the component into an `iced::Element` producing parent messages.
- `view` & `view_into`: Convenient aliases for `render` and `render_into`.

### `App`
Connects root models or components to Iced's application loop:
- Defines `boot`, `title`, `view`, and `update`.
- Customizes `subscription`, `theme`, and `scale_factor`.
- Features an integrated execution engine that recursively resolves batches, chains, and error handlers.

---

## Quick Start Example

```rust,no_run
use iced::widget::{button, column, text};
use iced::{Element, Task};
use iced_tea::{App, Signal};

#[derive(Debug, Clone)]
enum Message {
    Increment,
    Decrement,
}

struct CounterApp {
    count: i32,
}

impl App for CounterApp {
    type Message = Message;

    fn boot() -> (Self, Option<Task<Self::Message>>) {
        (Self { count: 0 }, None)
    }

    fn title(&self) -> String {
        format!("Counter: {}", self.count)
    }

    fn view(&self) -> Element<'_, Self::Message> {
        column![
            text(format!("Count: {}", self.count)).size(24),
            button("+").on_press(Message::Increment),
            button("-").on_press(Message::Decrement),
        ]
        .spacing(10)
        .into()
    }

    fn update(&mut self, message: Self::Message) -> anyhow::Result<Signal<Self::Message, ()>> {
        match message {
            Message::Increment => {
                self.count += 1;
                Signal::Done.ok()
            }
            Message::Decrement => {
                self.count -= 1;
                Signal::Done.ok()
            }
        }
    }
}

fn main() -> iced::Result {
    CounterApp::application().run()
}
```

---

## Installation

Add `iced-tea` as a git dependency to your `Cargo.toml` using the repository URL and release tag:

```toml
[dependencies]
iced = { version = "0.14.0", features = ["tokio"] }
iced-tea = { git = "https://github.com/LittleTealeaf/iced-tea.git", tag = "v0.1.0" }
```

Alternatively, you can add it via `cargo add`:

```bash
cargo add --git https://github.com/LittleTealeaf/iced-tea.git --tag v0.1.0 iced-tea
```

---

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
