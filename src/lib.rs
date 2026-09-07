//! # iced-tea
//!
//! Elm Architecture (TEA) extensions, effect combinators, and component traits for [Iced](https://iced.rs/).
//!
//! `iced-tea` provides structured abstractions to scale applications built with Iced:
//! - [`Signal`]: A unified effect and control-flow type replacing raw commands. Handles internal messages,
//!   parent events, async tasks, batches, chains, and error handling.
//! - [`Model`]: Hierarchical component state machines that transition on messages and yield signals.
//! - [`Component`]: Self-contained UI elements combining [`Model`] state with view rendering.
//! - [`App`]: Application runtime bridge hooking root components into Iced's event loop and window lifecycle.
//!
//! ## Quick Start Example
//!
//! ```no_run
//! use iced::widget::{button, column, text, Column};
//! use iced::{Element, Task};
//! use iced_tea::{App, Signal};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Increment,
//!     Decrement,
//! }
//!
//! struct CounterApp {
//!     count: i32,
//! }
//!
//! impl App for CounterApp {
//!     type Message = Message;
//!
//!     fn boot() -> (Self, Option<Task<Self::Message>>) {
//!         (Self { count: 0 }, None)
//!     }
//!
//!     fn title(&self) -> String {
//!         format!("Counter: {}", self.count)
//!     }
//!
//!     fn view(&self) -> Element<'_, Self::Message> {
//!         column![
//!             text(format!("Count: {}", self.count)),
//!             button("+").on_press(Message::Increment),
//!             button("-").on_press(Message::Decrement),
//!         ]
//!         .into()
//!     }
//!
//!     fn update(&mut self, message: Self::Message) -> anyhow::Result<Signal<Self::Message, ()>> {
//!         match message {
//!             Message::Increment => {
//!                 self.count += 1;
//!                 Signal::Done.ok()
//!             }
//!             Message::Decrement => {
//!                 self.count -= 1;
//!                 Signal::Done.ok()
//!             }
//!         }
//!     }
//! }
//!
//! fn main() -> iced::Result {
//!     CounterApp::application().run()
//! }
//! ```

pub mod app;
pub mod component;
pub mod model;
pub mod signal;

pub use app::App;
pub use component::Component;
pub use iced_futures::MaybeSend;
pub use model::{HandleMessage, Model};
pub use signal::Signal;

/// Backwards-compatibility type alias for [`Signal`].
pub type Effect<M, O> = Signal<M, O>;
